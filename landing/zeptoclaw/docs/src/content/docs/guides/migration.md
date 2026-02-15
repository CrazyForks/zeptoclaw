---
title: Migrating from OpenClaw
description: Step-by-step guide to migrate your OpenClaw setup to ZeptoClaw
---

If you're running OpenClaw today, ZeptoClaw gives you a lighter, faster, more secure runtime with most of the same integrations. This guide walks through what maps directly, what needs adaptation, and how to move your setup over.

## What you get

ZeptoClaw is a ground-up rewrite in Rust that keeps the best parts of OpenClaw's integration model while dropping the Node.js runtime:

- **4 MB binary** — no `node_modules`, no runtime dependencies
- **~50 ms startup** — cold start to first prompt
- **~6 MB RSS** — vs hundreds of MB for the Node process tree
- **Built-in safety layer** — prompt injection detection, secret leak scanning, policy engine
- **Apple Container support** — native macOS 15+ sandboxing alongside Docker

Most of the core concepts (skills, channels, tools, provider config) transfer directly. The main gaps are companion apps, voice features, and some channel-specific extensions.

## Before you start

1. **Install ZeptoClaw** — see the [installation guide](/docs/getting-started/installation/)
2. **Try the automated migration** — `zeptoclaw migrate` auto-detects your OpenClaw installation and converts config + skills:
   ```bash
   # Preview what would be migrated (no changes made)
   zeptoclaw migrate --dry-run

   # Run the migration interactively
   zeptoclaw migrate

   # Accept all defaults and specify a custom OpenClaw path
   zeptoclaw migrate --from ~/.openclaw --yes
   ```
   The command backs up your existing ZeptoClaw config before writing changes.
3. **Or migrate manually** — if you prefer, follow the field mapping below
4. **Locate your OpenClaw config** — typically `~/.openclaw/openclaw.json`
5. **Locate your OpenClaw skills** — typically `~/.openclaw/skills/` or the repo's `skills/` directory
6. **Back up your current setup** — `cp -r ~/.openclaw ~/.openclaw.bak`

## Config migration

> **Tip:** `zeptoclaw migrate` handles the config conversion automatically. The mapping below is for reference or manual migration.

OpenClaw uses `~/.openclaw/openclaw.json` (JSON5). ZeptoClaw uses `~/.zeptoclaw/config.json` (strict JSON). The structure is flatter and uses `snake_case` throughout.

### Field mapping

| OpenClaw | ZeptoClaw | Notes |
|----------|-----------|-------|
| `models.providers.<id>.baseUrl` | `providers.<id>.api_base` | snake_case |
| `models.providers.<id>.apiKey` | `providers.<id>.api_key` | snake_case |
| `agents.defaults.model.primary` | `providers.default` + `providers.<id>.model` | flat string, not nested object |
| `agents.defaults.workspace` | `agents.defaults.workspace` | same concept |
| `agents.defaults.contextTokens` | `compaction.context_limit` | moved to compaction section |
| `channels.telegram.token` | `channels.telegram.token` | same |
| `channels.discord.token` | `channels.discord.token` | same |
| `channels.slack.token` | `channels.slack.bot_token` | renamed |
| `session.scope` | — | ZeptoClaw uses container-per-request isolation instead |
| `tools.profile` | — | see tool approval gate below |
| `tools.web.search.provider` | — | Brave Search only (for now) |

### Before / after example

**OpenClaw** (`~/.openclaw/openclaw.json`):

```json5
{
  models: {
    providers: {
      anthropic: {
        apiKey: "sk-ant-...",
        baseUrl: "https://api.anthropic.com"
      },
      openai: {
        apiKey: "sk-...",
        baseUrl: "https://api.openai.com/v1"
      }
    }
  },
  agents: {
    defaults: {
      model: { primary: "claude-sonnet-4-5-20250929" },
      workspace: "~/projects",
      contextTokens: 100000
    }
  },
  channels: {
    telegram: { token: "123456:ABC..." },
    discord: { token: "MTIz..." }
  },
  session: { scope: "per-sender" }
}
```

**ZeptoClaw** (`~/.zeptoclaw/config.json`):

```json
{
  "providers": {
    "default": "anthropic",
    "anthropic": {
      "api_key": "sk-ant-...",
      "api_base": "https://api.anthropic.com",
      "model": "claude-sonnet-4-5-20250929"
    },
    "openai": {
      "api_key": "sk-...",
      "api_base": "https://api.openai.com/v1",
      "model": "gpt-5.1"
    }
  },
  "agents": {
    "defaults": {
      "agent_timeout_secs": 300
    }
  },
  "compaction": {
    "enabled": true,
    "context_limit": 100000
  },
  "channels": {
    "telegram": {
      "enabled": true,
      "token": "123456:ABC..."
    },
    "discord": {
      "enabled": true,
      "token": "MTIz..."
    }
  }
}
```

Key differences: flat provider config with `snake_case` fields, model set per-provider rather than globally, compaction is its own section, and channels have an explicit `enabled` flag.

You can validate your new config at any time:

```bash
zeptoclaw config check
```

## Skills migration

This is the easiest part. ZeptoClaw's skill loader is directly compatible with OpenClaw's skill format.

### Steps

The `zeptoclaw migrate` command copies skills automatically. To do it manually:

1. Copy your skills directory:
   ```bash
   cp -r ~/.openclaw/skills/* ~/.zeptoclaw/skills/
   ```

2. Verify they loaded:
   ```bash
   zeptoclaw skills list
   ```

### What works as-is

ZeptoClaw reads skills with the same YAML frontmatter and markdown body format. The loader checks metadata namespaces in this priority order: `zeptoclaw` → `clawdbot` → `openclaw` → raw (unnamespaced).

These skill features all carry over:
- `requires.bins` and `requires.anyBins` — binary dependency checks
- `requires.env` — environment variable requirements
- `os` — platform filtering (macos, linux)
- `{baseDir}` — path substitution to the skill's directory
- `always: true` — auto-inject into every conversation

### Fields silently ignored

These OpenClaw-specific fields are parsed but have no effect in ZeptoClaw:
- `requires.config` — config key dependencies
- `primaryEnv` — primary environment variable hint
- `skillKey` — explicit skill identifier
- `install` blocks — auto-install instructions

If a skill doesn't appear in `zeptoclaw skills list`, check that its YAML frontmatter is valid and that any `os` or `requires.bins` conditions are satisfied on your system.

## Plugin and extension migration

This requires more work. OpenClaw uses npm/TypeScript extensions; ZeptoClaw uses JSON manifest plugins.

### OpenClaw model

OpenClaw extensions live in `extensions/<name>/` with:
- `openclaw.plugin.json` — manifest with lifecycle hooks
- `config-schema.ts` — Zod-based config validation
- TypeScript implementation with full access to OpenClaw internals

### ZeptoClaw model

ZeptoClaw plugins are JSON files in `~/.zeptoclaw/plugins/<name>/plugin.json` with two execution modes:

- **Command mode** — shell command template with `{{param}}` interpolation
- **Binary mode** — JSON-RPC 2.0 over stdin/stdout

### Conversion example

**OpenClaw extension** (`extensions/github-pr/openclaw.plugin.json`):
```json
{
  "name": "github-pr",
  "version": "1.0.0",
  "main": "dist/index.js",
  "tools": [{
    "name": "create_pr",
    "description": "Create a GitHub pull request"
  }]
}
```

**ZeptoClaw plugin** (`~/.zeptoclaw/plugins/github-pr/plugin.json`):
```json
{
  "name": "github_pr",
  "description": "Create a GitHub pull request",
  "version": "1.0.0",
  "parameters": {
    "type": "object",
    "properties": {
      "title": { "type": "string", "description": "PR title" },
      "branch": { "type": "string", "description": "Source branch" }
    },
    "required": ["title", "branch"]
  },
  "command": "gh pr create --title {{title}} --head {{branch}} --body 'Created by ZeptoClaw'"
}
```

For complex extensions that need full programmatic control, use **binary mode** — write a small executable that speaks JSON-RPC 2.0 over stdin/stdout. See the [plugins guide](/docs/guides/plugins/) for details.

### Channel extensions

OpenClaw ships extensions for Signal, iMessage, Matrix, Mattermost, MS Teams, Feishu, and others. For channels not natively supported by ZeptoClaw, use the **webhook channel** with an external adapter:

1. Enable the webhook channel in your ZeptoClaw config
2. Run a lightweight bridge that converts the platform's API to HTTP POST requests
3. Point the bridge at ZeptoClaw's webhook endpoint

```json
{
  "channels": {
    "webhook": {
      "enabled": true,
      "port": 8080,
      "auth_token": "my-secret"
    }
  }
}
```

## Memory migration

The memory systems are architecturally different.

### OpenClaw memory

- Vector embeddings via QMD service
- Session export to files with retention policies
- Citation-based recall

### ZeptoClaw memory

- **Workspace memory** — BM25 keyword search over markdown files in your workspace
- **Long-term memory** — persistent key-value store at `~/.zeptoclaw/memory/longterm.json` with categories, tags, and access tracking

### Migration steps

1. **Export key memories** — If you relied on QMD for important context, export the most valuable entries to a `MEMORY.md` file in your workspace. ZeptoClaw's workspace memory tool will index it automatically.

2. **Use long-term memory for structured data** — For facts, preferences, and reference data, use the `longterm_memory` tool:
   ```
   Store this: my preferred language is Rust, category: preferences
   ```

3. **Session history** — ZeptoClaw maintains its own conversation history in `~/.zeptoclaw/sessions/`. Previous OpenClaw sessions won't carry over, but you can reference exported session files from your workspace.

## Tool mapping

Most core tools have direct equivalents with slightly different names.

| OpenClaw | ZeptoClaw | Notes |
|----------|-----------|-------|
| `exec` | `shell` | Same concept, different name |
| `read` | `read_file` | Same |
| `write` | `write_file` | Same |
| `edit` | `edit_file` | Same |
| `web-search` | `web_search` | Brave API in both |
| `web-fetch` | `web_fetch` | Same |
| `message` | `message` | Same |
| `cron` | `cron` | Same |
| `image-understanding` | — | Use an MCP server |
| `audio-understanding` | — | Use an MCP server |
| `browser` (Puppeteer/CDP) | — | Use an MCP server |
| `tts` | — | Not supported |
| `subagents` / `sessions-spawn` | `delegate` / `spawn` | Different API surface |

For tools without a built-in equivalent, ZeptoClaw's [MCP client](/docs/concepts/tools/) can connect to external tool servers. This is the recommended path for image understanding, audio processing, and browser automation.

OpenClaw's `tools.profile` system (per-tool execution policies) maps roughly to ZeptoClaw's **approval gate**:

```json
{
  "approval": {
    "enabled": true,
    "require_approval": ["shell", "write_file", "delegate"],
    "auto_approve": ["read_file", "memory", "web_search"]
  }
}
```

## What's not portable

Some OpenClaw features don't have ZeptoClaw equivalents:

- **Companion apps** (macOS, iOS, Android) — use channels (Telegram, Discord, etc.) instead
- **Voice features** (Wake Mode, Talk Mode, TTS) — not supported
- **OAuth provider flows** — API key authentication only
- **Per-agent sandbox overrides** — use global runtime config
- **DM pairing / `dmScope`** — use `allow_from` allowlists per channel
- **10+ channel extensions** (Signal, iMessage, Matrix, Line, IRC, etc.) — use the webhook adapter pattern
- **Config hot-reload** — restart the gateway after config changes
- **Gateway control UI** — no built-in web dashboard

## What you gain

Migrating isn't just about parity — ZeptoClaw adds capabilities that OpenClaw doesn't have:

- **Safety layer** — prompt injection detection (Aho-Corasick + regex), secret leak scanning (22 patterns), and a 7-rule policy engine — all enabled by default
- **Tool approval gate** — policy-based gating for sensitive tool calls
- **Circuit breaker** — automatic failover with retry and fallback provider stacks
- **Token budget** — per-session cost control with configurable limits
- **Apple Container isolation** — native macOS 15+ sandboxing without Docker
- **Agent templates** — preconfigured roles (coder, researcher, writer, analyst) with tool whitelists
- **Persistent reminders** — scheduled reminders with cron-based delivery
- **Prometheus telemetry** — built-in metrics export for monitoring
- **Batch mode** — process multiple prompts from a file
- **Cost tracking** — per-provider, per-model cost accumulation with warnings

## Troubleshooting

### Skill not loading

```bash
zeptoclaw skills list
```

- Verify the file is named `SKILL.md` with valid YAML frontmatter
- Check `os` filter matches your platform
- Check `requires.bins` dependencies are installed
- Check `requires.env` variables are set

### Provider errors

```bash
# Quick smoke test
zeptoclaw agent -m "hello"
```

- Verify `api_key` is set in config or via environment variable
- Check `providers.default` points to a configured provider
- Run `zeptoclaw config check` for validation errors

### Missing tool

- Check if the tool needs an MCP server (image, audio, browser)
- Check if it's a plugin that needs to be converted (see plugin migration above)
- Run `zeptoclaw agent -m "list your tools"` to see what's available

### Config validation errors

```bash
zeptoclaw config check
```

- OpenClaw uses JSON5 (trailing commas, comments) — ZeptoClaw requires strict JSON
- All keys must be `snake_case`
- Remove any OpenClaw-specific sections (`session.scope`, `tools.profile`, `gateway.controlUi`, etc.)
