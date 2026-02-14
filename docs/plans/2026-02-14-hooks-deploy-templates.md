# Hooks System + Cloud Deployment Templates

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a config-driven hooks system (before_tool, after_tool, on_error) and ready-to-use cloud deployment templates.

**Architecture:** Hooks are config-driven (JSON rules, not Rust traits) matching ZeptoClaw's existing pattern (see ApprovalGate). The HookEngine evaluates rules at 3 points in the agent loop. Deploy templates are static config files in `deploy/`.

**Tech Stack:** Rust (serde, tracing, Arc), YAML/TOML for deploy configs

---

## Part A: Hooks System

### Task 1: Create HooksConfig types

**Files:**
- Create: `src/hooks/mod.rs`

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hooks_config_default() {
        let config = HooksConfig::default();
        assert!(!config.enabled);
        assert!(config.before_tool.is_empty());
        assert!(config.after_tool.is_empty());
        assert!(config.on_error.is_empty());
    }

    #[test]
    fn test_hooks_config_deserialize() {
        let json = r#"{
            "enabled": true,
            "before_tool": [
                { "action": "log", "tools": ["shell"], "level": "warn" }
            ]
        }"#;
        let config: HooksConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert_eq!(config.before_tool.len(), 1);
        assert_eq!(config.before_tool[0].action, HookAction::Log);
    }

    #[test]
    fn test_hook_rule_matches_tool() {
        let rule = HookRule {
            action: HookAction::Log,
            tools: vec!["shell".to_string()],
            channels: vec![],
            level: Some("warn".to_string()),
            message: None,
            channel: None,
            chat_id: None,
        };
        assert!(rule.matches_tool("shell"));
        assert!(!rule.matches_tool("echo"));
    }

    #[test]
    fn test_hook_rule_wildcard_matches_all() {
        let rule = HookRule {
            action: HookAction::Log,
            tools: vec!["*".to_string()],
            channels: vec![],
            level: None,
            message: None,
            channel: None,
            chat_id: None,
        };
        assert!(rule.matches_tool("shell"));
        assert!(rule.matches_tool("echo"));
        assert!(rule.matches_tool("anything"));
    }

    #[test]
    fn test_hook_rule_matches_channel() {
        let rule = HookRule {
            action: HookAction::Block,
            tools: vec!["shell".to_string()],
            channels: vec!["telegram".to_string()],
            level: None,
            message: Some("blocked".to_string()),
            channel: None,
            chat_id: None,
        };
        assert!(rule.matches_channel("telegram"));
        assert!(!rule.matches_channel("discord"));
    }

    #[test]
    fn test_hook_rule_empty_channels_matches_all() {
        let rule = HookRule {
            action: HookAction::Log,
            tools: vec!["*".to_string()],
            channels: vec![],
            level: None,
            message: None,
            channel: None,
            chat_id: None,
        };
        assert!(rule.matches_channel("telegram"));
        assert!(rule.matches_channel("discord"));
        assert!(rule.matches_channel("cli"));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test hooks::tests --lib -- --nocapture`
Expected: FAIL (module doesn't exist)

**Step 3: Write implementation**

```rust
//! Hook system for ZeptoClaw agent loop.
//!
//! Config-driven hooks that fire at specific points:
//! - `before_tool` — before tool execution (can log or block)
//! - `after_tool` — after tool execution (can log or notify)
//! - `on_error` — when a tool fails (can log or notify)

use serde::{Deserialize, Serialize};

/// What a hook rule does when triggered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookAction {
    /// Log the event via tracing.
    Log,
    /// Block the tool from executing (before_tool only).
    Block,
    /// Send a notification to a channel.
    Notify,
}

/// A single hook rule that matches tool calls and performs an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HookRule {
    /// Action to perform.
    pub action: HookAction,
    /// Tool names to match. `["*"]` matches all tools. Empty = match none.
    pub tools: Vec<String>,
    /// Channel names to match. Empty = match all channels.
    pub channels: Vec<String>,
    /// Log level for `Log` action (trace/debug/info/warn/error).
    pub level: Option<String>,
    /// Custom message for `Block` action.
    pub message: Option<String>,
    /// Target channel name for `Notify` action.
    pub channel: Option<String>,
    /// Target chat ID for `Notify` action.
    pub chat_id: Option<String>,
}

impl Default for HookRule {
    fn default() -> Self {
        Self {
            action: HookAction::Log,
            tools: vec![],
            channels: vec![],
            level: None,
            message: None,
            channel: None,
            chat_id: None,
        }
    }
}

impl HookRule {
    /// Check if this rule matches the given tool name.
    pub fn matches_tool(&self, tool_name: &str) -> bool {
        self.tools.iter().any(|t| t == "*" || t == tool_name)
    }

    /// Check if this rule matches the given channel name.
    /// Empty channels list means match all.
    pub fn matches_channel(&self, channel_name: &str) -> bool {
        self.channels.is_empty()
            || self.channels.iter().any(|c| c == "*" || c == channel_name)
    }
}

/// Hooks configuration for `config.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HooksConfig {
    /// Master switch for hooks.
    pub enabled: bool,
    /// Rules evaluated before each tool execution.
    pub before_tool: Vec<HookRule>,
    /// Rules evaluated after each tool execution.
    pub after_tool: Vec<HookRule>,
    /// Rules evaluated when a tool returns an error.
    pub on_error: Vec<HookRule>,
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test hooks::tests --lib -- --nocapture`
Expected: PASS (all 5 tests)

**Step 5: Commit**

```bash
git add src/hooks/mod.rs
git commit -m "feat(hooks): add HooksConfig types and HookRule matching"
```

---

### Task 2: Create HookEngine runtime

**Files:**
- Modify: `src/hooks/mod.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn test_hook_engine_disabled_does_nothing() {
    let config = HooksConfig::default(); // enabled: false
    let engine = HookEngine::new(config);
    let result = engine.before_tool("shell", &serde_json::json!({}), "telegram");
    assert_eq!(result, HookResult::Continue);
}

#[test]
fn test_hook_engine_before_tool_log() {
    let config = HooksConfig {
        enabled: true,
        before_tool: vec![HookRule {
            action: HookAction::Log,
            tools: vec!["shell".to_string()],
            level: Some("warn".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let engine = HookEngine::new(config);
    let result = engine.before_tool("shell", &serde_json::json!({"cmd": "ls"}), "cli");
    assert_eq!(result, HookResult::Continue);
}

#[test]
fn test_hook_engine_before_tool_block() {
    let config = HooksConfig {
        enabled: true,
        before_tool: vec![HookRule {
            action: HookAction::Block,
            tools: vec!["shell".to_string()],
            channels: vec!["telegram".to_string()],
            message: Some("Shell disabled on Telegram".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let engine = HookEngine::new(config);

    // Should block shell on telegram
    let result = engine.before_tool("shell", &serde_json::json!({}), "telegram");
    assert!(matches!(result, HookResult::Block(_)));

    // Should NOT block shell on CLI
    let result = engine.before_tool("shell", &serde_json::json!({}), "cli");
    assert_eq!(result, HookResult::Continue);

    // Should NOT block echo on telegram
    let result = engine.before_tool("echo", &serde_json::json!({}), "telegram");
    assert_eq!(result, HookResult::Continue);
}

#[test]
fn test_hook_engine_after_tool() {
    let config = HooksConfig {
        enabled: true,
        after_tool: vec![HookRule {
            action: HookAction::Log,
            tools: vec!["*".to_string()],
            level: Some("info".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let engine = HookEngine::new(config);
    // after_tool just logs, doesn't return block/continue
    engine.after_tool("shell", "result text", std::time::Duration::from_millis(50), "cli");
}

#[test]
fn test_hook_engine_on_error() {
    let config = HooksConfig {
        enabled: true,
        on_error: vec![HookRule {
            action: HookAction::Log,
            tools: vec!["*".to_string()],
            level: Some("error".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let engine = HookEngine::new(config);
    engine.on_error("shell", "command not found", "cli");
}

#[test]
fn test_hook_engine_multiple_rules_first_block_wins() {
    let config = HooksConfig {
        enabled: true,
        before_tool: vec![
            HookRule {
                action: HookAction::Log,
                tools: vec!["*".to_string()],
                level: Some("info".to_string()),
                ..Default::default()
            },
            HookRule {
                action: HookAction::Block,
                tools: vec!["shell".to_string()],
                message: Some("blocked".to_string()),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let engine = HookEngine::new(config);
    let result = engine.before_tool("shell", &serde_json::json!({}), "cli");
    assert!(matches!(result, HookResult::Block(_)));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test hooks::tests --lib -- --nocapture`
Expected: FAIL (HookEngine doesn't exist)

**Step 3: Write implementation**

```rust
/// Result of evaluating before_tool hooks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookResult {
    /// Allow the tool to execute.
    Continue,
    /// Block the tool with the given message.
    Block(String),
}

/// Runtime hook engine that evaluates rules from HooksConfig.
pub struct HookEngine {
    config: HooksConfig,
}

impl HookEngine {
    /// Create a new HookEngine from configuration.
    pub fn new(config: HooksConfig) -> Self {
        Self { config }
    }

    /// Evaluate before_tool hooks. Returns Block if any rule blocks.
    pub fn before_tool(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
        channel: &str,
    ) -> HookResult {
        if !self.config.enabled {
            return HookResult::Continue;
        }

        for rule in &self.config.before_tool {
            if !rule.matches_tool(tool_name) || !rule.matches_channel(channel) {
                continue;
            }

            match rule.action {
                HookAction::Log => {
                    let level = rule.level.as_deref().unwrap_or("info");
                    match level {
                        "error" => tracing::error!(hook = "before_tool", tool = tool_name, channel = channel, "Hook: tool call"),
                        "warn" => tracing::warn!(hook = "before_tool", tool = tool_name, channel = channel, "Hook: tool call"),
                        "debug" => tracing::debug!(hook = "before_tool", tool = tool_name, channel = channel, "Hook: tool call"),
                        "trace" => tracing::trace!(hook = "before_tool", tool = tool_name, channel = channel, "Hook: tool call"),
                        _ => tracing::info!(hook = "before_tool", tool = tool_name, channel = channel, "Hook: tool call"),
                    }
                }
                HookAction::Block => {
                    let msg = rule
                        .message
                        .clone()
                        .unwrap_or_else(|| format!("Tool '{}' blocked by hook", tool_name));
                    tracing::info!(hook = "before_tool", tool = tool_name, channel = channel, "Hook: blocking tool");
                    return HookResult::Block(msg);
                }
                HookAction::Notify => {
                    // Notify is not useful in before_tool (no bus access here).
                    // Log instead.
                    tracing::info!(hook = "before_tool", tool = tool_name, channel = channel, "Hook: notify (logged)");
                }
            }
        }

        HookResult::Continue
    }

    /// Evaluate after_tool hooks (logging/notification only).
    pub fn after_tool(
        &self,
        tool_name: &str,
        _result: &str,
        elapsed: std::time::Duration,
        channel: &str,
    ) {
        if !self.config.enabled {
            return;
        }

        for rule in &self.config.after_tool {
            if !rule.matches_tool(tool_name) || !rule.matches_channel(channel) {
                continue;
            }

            match rule.action {
                HookAction::Log => {
                    let level = rule.level.as_deref().unwrap_or("info");
                    let ms = elapsed.as_millis();
                    match level {
                        "error" => tracing::error!(hook = "after_tool", tool = tool_name, latency_ms = %ms, "Hook: tool completed"),
                        "warn" => tracing::warn!(hook = "after_tool", tool = tool_name, latency_ms = %ms, "Hook: tool completed"),
                        "debug" => tracing::debug!(hook = "after_tool", tool = tool_name, latency_ms = %ms, "Hook: tool completed"),
                        _ => tracing::info!(hook = "after_tool", tool = tool_name, latency_ms = %ms, "Hook: tool completed"),
                    }
                }
                HookAction::Block => {
                    // Block doesn't make sense in after_tool; ignore.
                }
                HookAction::Notify => {
                    tracing::info!(hook = "after_tool", tool = tool_name, "Hook: notify (logged)");
                }
            }
        }
    }

    /// Evaluate on_error hooks (logging/notification only).
    pub fn on_error(&self, tool_name: &str, error: &str, channel: &str) {
        if !self.config.enabled {
            return;
        }

        for rule in &self.config.on_error {
            if !rule.matches_tool(tool_name) || !rule.matches_channel(channel) {
                continue;
            }

            match rule.action {
                HookAction::Log => {
                    let level = rule.level.as_deref().unwrap_or("error");
                    match level {
                        "warn" => tracing::warn!(hook = "on_error", tool = tool_name, error = error, "Hook: tool error"),
                        "debug" => tracing::debug!(hook = "on_error", tool = tool_name, error = error, "Hook: tool error"),
                        _ => tracing::error!(hook = "on_error", tool = tool_name, error = error, "Hook: tool error"),
                    }
                }
                HookAction::Block => {
                    // Block doesn't make sense in on_error; ignore.
                }
                HookAction::Notify => {
                    tracing::info!(hook = "on_error", tool = tool_name, error = error, "Hook: error notify (logged)");
                }
            }
        }
    }

    /// Whether hooks are enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test hooks::tests --lib -- --nocapture`
Expected: PASS (all 11 tests)

**Step 5: Commit**

```bash
git add src/hooks/mod.rs
git commit -m "feat(hooks): add HookEngine with before_tool, after_tool, on_error"
```

---

### Task 3: Wire hooks into Config and lib.rs

**Files:**
- Modify: `src/config/types.rs` — add `hooks: HooksConfig` to Config
- Modify: `src/lib.rs` — add `pub mod hooks;`
- Modify: `src/config/mod.rs` — add env override for `ZEPTOCLAW_HOOKS_ENABLED`

**Step 1: Write the failing test**

```rust
// In src/hooks/mod.rs tests
#[test]
fn test_hooks_config_in_full_config() {
    let json = r#"{"hooks": {"enabled": true, "before_tool": [{"action": "log", "tools": ["*"]}]}}"#;
    let config: zeptoclaw::config::Config = serde_json::from_str(json).unwrap();
    assert!(config.hooks.enabled);
    assert_eq!(config.hooks.before_tool.len(), 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_hooks_config_in_full_config --lib`
Expected: FAIL (no `hooks` field on Config)

**Step 3: Implement**

In `src/config/types.rs`, add to Config struct:
```rust
/// Hook system configuration
pub hooks: crate::hooks::HooksConfig,
```

In `src/lib.rs`, add:
```rust
pub mod hooks;
```

In `src/config/mod.rs`, add env override:
```rust
// In apply_env_overrides():
if let Ok(v) = std::env::var("ZEPTOCLAW_HOOKS_ENABLED") {
    config.hooks.enabled = v.eq_ignore_ascii_case("true") || v == "1";
}
```

**Step 4: Run tests**

Run: `cargo test --lib`
Expected: PASS

**Step 5: Commit**

```bash
git add src/config/types.rs src/lib.rs src/config/mod.rs src/hooks/mod.rs
git commit -m "feat(hooks): wire HooksConfig into Config and lib.rs"
```

---

### Task 4: Wire HookEngine into agent loop

**Files:**
- Modify: `src/agent/loop.rs` — create HookEngine, call at 3 points

**Step 1: Understand insertion points**

The tool execution happens at lines 362-424 in loop.rs:
- **Before tool** (line ~385, before approval gate check): Add `hook_engine.before_tool()` check
- **After tool success** (line ~400): Add `hook_engine.after_tool()` call
- **On tool error** (line ~409): Add `hook_engine.on_error()` call

**Step 2: Implement**

At the top of `process_message()` or wherever the agent loop struct is initialized, create the HookEngine:

```rust
let hook_engine = crate::hooks::HookEngine::new(self.config.hooks.clone());
```

In the tool execution closure (line ~376), before the approval gate check:

```rust
// Check hooks before executing
if hook_engine.is_enabled() {
    let hook_result = hook_engine.before_tool(&name, &args, ctx.channel());
    if let crate::hooks::HookResult::Block(msg) = hook_result {
        return (id, format!("Tool '{}' blocked by hook: {}", name, msg));
    }
}
```

After successful tool execution (line ~400):

```rust
hook_engine.after_tool(&name, &r, elapsed, ctx.channel());
```

On tool error (line ~409):

```rust
hook_engine.on_error(&name, &e.to_string(), ctx.channel());
```

**Step 3: Run tests**

Run: `cargo test --lib`
Expected: PASS

**Step 4: Commit**

```bash
git add src/agent/loop.rs
git commit -m "feat(hooks): wire HookEngine into agent loop at 3 points"
```

---

### Task 5: Add hooks to config validation

**Files:**
- Modify: `src/config/validate.rs` — add "hooks" to KNOWN_TOP_LEVEL

**Step 1: Add "hooks" to known keys**

Find `KNOWN_TOP_LEVEL` array and add `"hooks"`.

**Step 2: Run tests**

Run: `cargo test validate --lib`
Expected: PASS

**Step 3: Commit**

```bash
git add src/config/validate.rs
git commit -m "feat(hooks): add hooks to config validation known keys"
```

---

## Part B: Cloud Deployment Templates

### Task 6: Create deploy/ directory with templates

**Files:**
- Create: `deploy/docker-compose.single.yml`
- Create: `deploy/fly.toml`
- Create: `deploy/railway.json`
- Create: `deploy/render.yaml`
- Create: `deploy/.env.example`

**Step 1: Create docker-compose.single.yml**

```yaml
# ZeptoClaw Single-Tenant Deployment
#
# The simplest way to run ZeptoClaw on a VPS.
#
# Setup:
#   1. Build: docker build -t zeptoclaw ..
#   2. Copy: cp .env.example .env && edit .env
#   3. Start: docker compose -f docker-compose.single.yml up -d
#   4. Logs: docker compose logs -f

services:
  zeptoclaw:
    image: zeptoclaw:latest
    container_name: zeptoclaw
    restart: unless-stopped
    volumes:
      - zeptoclaw-data:/data
    env_file: .env
    environment:
      - RUST_LOG=zeptoclaw=info
      - RUST_LOG_FORMAT=json
    ports:
      - "8080:8080"   # Gateway API
      - "9090:9090"   # Health check
    deploy:
      resources:
        limits:
          memory: 128M
          cpus: "0.5"
    healthcheck:
      test: ["CMD-SHELL", "wget -qO- http://localhost:9090/healthz || exit 1"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
    logging:
      driver: json-file
      options:
        max-size: "10m"
        max-file: "3"
    command: ["zeptoclaw", "gateway"]

volumes:
  zeptoclaw-data:
```

**Step 2: Create fly.toml**

```toml
# ZeptoClaw on Fly.io
#
# Deploy:
#   1. fly launch --no-deploy
#   2. fly secrets set ZEPTOCLAW_PROVIDERS_ANTHROPIC_API_KEY=sk-ant-...
#   3. fly secrets set ZEPTOCLAW_CHANNELS_TELEGRAM_BOT_TOKEN=...
#   4. fly deploy
#
# Docs: https://fly.io/docs/rust/

app = "zeptoclaw"
primary_region = "sin"  # Singapore (change to nearest region)

[build]
  dockerfile = "../Dockerfile"

[env]
  RUST_LOG = "zeptoclaw=info"
  RUST_LOG_FORMAT = "json"

[http_service]
  internal_port = 8080
  force_https = true
  auto_stop_machines = "stop"
  auto_start_machines = true
  min_machines_running = 0

[[services]]
  protocol = "tcp"
  internal_port = 8080

  [[services.ports]]
    port = 443
    handlers = ["tls", "http"]

  [[services.tcp_checks]]
    grace_period = "10s"
    interval = "30s"
    timeout = "5s"

[[vm]]
  memory = "256mb"
  cpu_kind = "shared"
  cpus = 1

[mounts]
  source = "zeptoclaw_data"
  destination = "/data"
```

**Step 3: Create railway.json**

```json
{
  "$schema": "https://railway.com/railway.schema.json",
  "build": {
    "dockerfilePath": "../Dockerfile"
  },
  "deploy": {
    "startCommand": "zeptoclaw gateway",
    "healthcheckPath": "/healthz",
    "healthcheckTimeout": 5,
    "restartPolicyType": "ON_FAILURE",
    "restartPolicyMaxRetries": 3
  }
}
```

**Step 4: Create render.yaml**

```yaml
# ZeptoClaw on Render
#
# Deploy:
#   1. Push to GitHub
#   2. Connect repo in Render dashboard
#   3. Set environment variables
#   4. Deploy

services:
  - type: web
    name: zeptoclaw
    runtime: docker
    dockerfilePath: ../Dockerfile
    dockerCommand: zeptoclaw gateway
    healthCheckPath: /healthz
    envVars:
      - key: RUST_LOG
        value: zeptoclaw=info
      - key: RUST_LOG_FORMAT
        value: json
      - key: ZEPTOCLAW_PROVIDERS_ANTHROPIC_API_KEY
        sync: false
      - key: ZEPTOCLAW_CHANNELS_TELEGRAM_BOT_TOKEN
        sync: false
    disk:
      name: zeptoclaw-data
      mountPath: /data
      sizeGB: 1
    plan: starter
    region: singapore
```

**Step 5: Create .env.example**

```bash
# ZeptoClaw Environment Configuration
# Copy this file to .env and fill in your values.

# === LLM Provider (pick one) ===
ZEPTOCLAW_PROVIDERS_ANTHROPIC_API_KEY=
# ZEPTOCLAW_PROVIDERS_OPENAI_API_KEY=

# === Channel (pick one or more) ===
ZEPTOCLAW_CHANNELS_TELEGRAM_BOT_TOKEN=

# === Optional: Web Search ===
# BRAVE_API_KEY=

# === Optional: WhatsApp ===
# ZEPTOCLAW_TOOLS_WHATSAPP_PHONE_NUMBER_ID=
# ZEPTOCLAW_TOOLS_WHATSAPP_ACCESS_TOKEN=

# === Optional: Google Sheets ===
# ZEPTOCLAW_TOOLS_GOOGLE_SHEETS_ACCESS_TOKEN=

# === Logging ===
RUST_LOG=zeptoclaw=info
RUST_LOG_FORMAT=json
```

**Step 6: Commit**

```bash
git add deploy/
git commit -m "feat(deploy): add cloud deployment templates (Docker, Fly.io, Railway, Render)"
```

---

### Task 7: Move existing multi-tenant compose into deploy/

**Files:**
- Move: `docker-compose.multi-tenant.yml` → `deploy/docker-compose.multi.yml`
- Update references if any

**Step 1: Move file**

```bash
git mv docker-compose.multi-tenant.yml deploy/docker-compose.multi.yml
```

**Step 2: Commit**

```bash
git commit -m "refactor(deploy): move multi-tenant compose into deploy/"
```

---

## Summary

| Task | Description | Estimated Tests |
|------|------------|-----------------|
| 1 | HooksConfig types + HookRule matching | 5 tests |
| 2 | HookEngine runtime (before/after/error) | 6 tests |
| 3 | Wire into Config + lib.rs | 1 test |
| 4 | Wire into agent loop | 0 new (existing pass) |
| 5 | Config validation | 0 new (existing pass) |
| 6 | Deploy templates | 0 (config files) |
| 7 | Move multi-tenant compose | 0 (file move) |

**Total: ~12 new tests, 7 tasks, ~200 lines of Rust, 5 deploy config files**
