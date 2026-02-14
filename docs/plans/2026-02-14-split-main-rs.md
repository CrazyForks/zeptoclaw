# Split main.rs into cli/ Module — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Break the 2,212-line `src/main.rs` into a `src/cli/` module with focused submodules, keeping `main.rs` under 50 lines.

**Architecture:** Extract CLI types, command handlers, onboarding wizard, agent factory, and shared helpers into `src/cli/` submodules. `main.rs` retains only logging init + clap parse + dispatch. Shared helpers (`expand_tilde`, `read_line`, `create_agent`, etc.) go into a `common.rs` submodule used by multiple commands.

**Tech Stack:** Rust, clap (derive), anyhow, tracing

**Current state:** All 2,212 lines live in `src/main.rs`. No `src/cli/` directory exists yet.

---

## File Layout (Target)

```
src/
├── main.rs              (~40 lines: logging init, parse, dispatch)
├── cli/
│   ├── mod.rs           (pub mod + CLI struct + Commands enum + subcommand enums)
│   ├── common.rs        (shared helpers: expand_tilde, read_line, read_secret, create_agent, skills helpers)
│   ├── onboard.rs       (cmd_onboard + all configure_* functions + label helpers)
│   ├── agent.rs         (cmd_agent + cmd_agent_stdin)
│   ├── gateway.rs       (cmd_gateway + validate_docker/apple + configured_docker_binary)
│   ├── status.rs        (cmd_status + cmd_auth + cmd_auth_status)
│   ├── heartbeat.rs     (cmd_heartbeat + heartbeat_file_path)
│   ├── skills.rs        (cmd_skills)
│   └── config.rs        (cmd_config)
```

## Dependency Map

These helpers are used across multiple command files:

| Helper | Used by |
|--------|---------|
| `create_agent()` | agent.rs, gateway.rs, heartbeat.rs |
| `expand_tilde()` | common.rs (used by heartbeat_file_path, skills_loader_from_config) |
| `read_line()` | onboard.rs only |
| `read_secret()` | onboard.rs only |
| `build_skills_prompt()` | common.rs (used by create_agent) |
| `skills_loader_from_config()` | common.rs (used by build_skills_prompt, cmd_skills, cmd_status) |
| `escape_xml()` | common.rs (used by build_skills_prompt) |
| `memory_backend_label()` | onboard.rs, status.rs |
| `memory_citations_label()` | onboard.rs, status.rs |
| `heartbeat_file_path()` | heartbeat.rs, status.rs (move to heartbeat.rs, re-export) |

---

## Task 1: Create cli/mod.rs with CLI types

**Files:**
- Create: `src/cli/mod.rs`

**Step 1: Create the file**

Create `src/cli/mod.rs` containing:
- All `pub mod` declarations (common, onboard, agent, gateway, status, heartbeat, skills, config)
- The `Cli` struct with `#[derive(Parser)]`
- The `Commands` enum with `#[derive(Subcommand)]`
- The `SkillsAction`, `AuthAction`, `ConfigAction` sub-enums
- A `pub async fn run()` function that does logging init + parse + match dispatch

Copy lines 1-193 from current `main.rs` into this file, adjusting function calls to reference sibling modules (e.g., `onboard::cmd_onboard()`, `agent::cmd_agent()`, etc.).

The `run()` function should contain what's currently in `main()` (lines 140-193) but call into submodules:

```rust
pub async fn run() -> Result<()> {
    // logging init (lines 142-155)
    // ...
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Version) | None => version::cmd_version(), // or inline
        Some(Commands::Onboard) => onboard::cmd_onboard().await?,
        Some(Commands::Agent { message, stream }) => agent::cmd_agent(message, stream).await?,
        Some(Commands::Gateway { containerized }) => gateway::cmd_gateway(containerized).await?,
        Some(Commands::AgentStdin) => agent::cmd_agent_stdin().await?,
        Some(Commands::Heartbeat { show, edit }) => heartbeat::cmd_heartbeat(show, edit).await?,
        Some(Commands::Skills { action }) => skills::cmd_skills(action).await?,
        Some(Commands::Auth { action }) => status::cmd_auth(action).await?,
        Some(Commands::Status) => status::cmd_status().await?,
        Some(Commands::Config { action }) => config::cmd_config(action).await?,
    }
    Ok(())
}
```

**Step 2: Verify it compiles (will fail — that's expected)**

This task just creates the skeleton. Subsequent tasks fill in each submodule.

**Step 3: Commit**

```bash
git add src/cli/mod.rs
git commit -m "refactor: create cli module skeleton with CLI types and dispatch"
```

---

## Task 2: Create cli/common.rs with shared helpers

**Files:**
- Create: `src/cli/common.rs`

**Step 1: Move shared helpers**

Move these functions from `main.rs` to `src/cli/common.rs`:

- `read_line()` (line 250-257)
- `read_secret()` (line 260-263)
- `expand_tilde()` (lines 1588-1599)
- `create_agent()` (lines 793-995) — this is the big one, ~200 lines
- `skills_loader_from_config()` (lines 997-1005)
- `build_skills_prompt()` (lines 1007-1071)
- `escape_xml()` (lines 1073-1078)
- `memory_backend_label()` (lines 481-487)
- `memory_citations_label()` (lines 489-495)

All functions should be `pub(crate)`.

Add necessary imports at the top:

```rust
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::{error, info, warn};

use zeptoclaw::agent::{AgentLoop, ContextBuilder};
use zeptoclaw::bus::MessageBus;
use zeptoclaw::config::{Config, MemoryBackend, MemoryCitationsMode};
use zeptoclaw::cron::CronService;
use zeptoclaw::providers::{
    resolve_runtime_provider, ClaudeProvider, OpenAIProvider,
};
use zeptoclaw::runtime::{create_runtime, NativeRuntime};
use zeptoclaw::session::SessionManager;
use zeptoclaw::skills::SkillsLoader;
use zeptoclaw::tools::cron::CronTool;
use zeptoclaw::tools::delegate::DelegateTool;
use zeptoclaw::tools::filesystem::{EditFileTool, ListDirTool, ReadFileTool, WriteFileTool};
use zeptoclaw::tools::shell::ShellTool;
use zeptoclaw::tools::spawn::SpawnTool;
use zeptoclaw::tools::{
    EchoTool, GoogleSheetsTool, MemoryGetTool, MemorySearchTool, MessageTool, R8rTool,
    WebFetchTool, WebSearchTool, WhatsAppTool,
};
```

**Step 2: Commit**

```bash
git add src/cli/common.rs
git commit -m "refactor: extract shared CLI helpers into cli/common.rs"
```

---

## Task 3: Create cli/onboard.rs

**Files:**
- Create: `src/cli/onboard.rs`

**Step 1: Move onboarding functions**

Move these from `main.rs`:

- `cmd_onboard()` (lines 266-370)
- `configure_web_search()` (lines 373-410)
- `configure_memory()` (lines 413-479)
- `configure_whatsapp_tool()` (lines 498-535)
- `configure_google_sheets_tool()` (lines 538-564)
- `configure_heartbeat()` (lines 567-605)
- `configure_anthropic()` (lines 608-634)
- `configure_openai()` (lines 637-682)
- `configure_telegram()` (lines 685-710)
- `configure_runtime()` (lines 713-790)

These use `read_line()`, `read_secret()`, `memory_backend_label()`, `memory_citations_label()` — import from `super::common`.

Only `cmd_onboard` needs to be `pub(crate)`, the configure_* functions stay private.

**Step 2: Commit**

```bash
git add src/cli/onboard.rs
git commit -m "refactor: extract onboarding wizard into cli/onboard.rs"
```

---

## Task 4: Create cli/agent.rs

**Files:**
- Create: `src/cli/agent.rs`

**Step 1: Move agent commands**

Move these from `main.rs`:

- `cmd_agent()` (lines 1081-1236)
- `cmd_agent_stdin()` (lines 1242-1302)

These use `create_agent()` from `super::common`.

Both functions should be `pub(crate)`.

**Step 2: Commit**

```bash
git add src/cli/agent.rs
git commit -m "refactor: extract agent commands into cli/agent.rs"
```

---

## Task 5: Create cli/gateway.rs

**Files:**
- Create: `src/cli/gateway.rs`

**Step 1: Move gateway command**

Move these from `main.rs`:

- `cmd_gateway()` (lines 1305-1577) — the biggest command handler at ~270 lines
- `validate_docker_available()` (lines 1919-1927)
- `configured_docker_binary()` (lines 1929-1936)
- `validate_apple_available()` (lines 1940-1947) — `#[cfg(target_os = "macos")]`

These use `create_agent()`, `heartbeat_file_path()` from sibling modules.

Only `cmd_gateway` needs to be `pub(crate)`.

Import `heartbeat_file_path` from `super::heartbeat`.

**Step 2: Commit**

```bash
git add src/cli/gateway.rs
git commit -m "refactor: extract gateway command into cli/gateway.rs"
```

---

## Task 6: Create cli/heartbeat.rs

**Files:**
- Create: `src/cli/heartbeat.rs`

**Step 1: Move heartbeat command**

Move these from `main.rs`:

- `cmd_heartbeat()` (lines 1602-1642)
- `heartbeat_file_path()` (lines 1579-1586) — make `pub(crate)` since gateway.rs and status.rs use it

These use `create_agent()` and `expand_tilde()` from `super::common`.

**Step 2: Commit**

```bash
git add src/cli/heartbeat.rs
git commit -m "refactor: extract heartbeat command into cli/heartbeat.rs"
```

---

## Task 7: Create cli/skills.rs

**Files:**
- Create: `src/cli/skills.rs`

**Step 1: Move skills command**

Move `cmd_skills()` (lines 1645-1718) from `main.rs`.

Uses `skills_loader_from_config()` from `super::common`.

Takes `SkillsAction` from `super::SkillsAction`.

**Step 2: Commit**

```bash
git add src/cli/skills.rs
git commit -m "refactor: extract skills command into cli/skills.rs"
```

---

## Task 8: Create cli/status.rs

**Files:**
- Create: `src/cli/status.rs`

**Step 1: Move status and auth commands**

Move these from `main.rs`:

- `cmd_status()` (lines 1950-2212)
- `cmd_auth()` (lines 1721-1741)
- `cmd_auth_status()` (lines 1744-1916)

Uses `memory_backend_label()`, `memory_citations_label()`, `skills_loader_from_config()`, `heartbeat_file_path()` from siblings.

Takes `AuthAction` from `super::AuthAction`.

**Bonus refactor:** The provider status checking in `cmd_auth_status()` repeats the same 10-line pattern 6 times. Refactor into a helper:

```rust
fn provider_status(provider: &Option<ProviderConfig>) -> &'static str {
    provider
        .as_ref()
        .and_then(|p| p.api_key.as_ref())
        .map(|k| if k.is_empty() { "not set" } else { "configured" })
        .unwrap_or("not set")
}
```

Then:
```rust
println!("  Anthropic (Claude): {}", provider_status(&config.providers.anthropic));
println!("  OpenAI:             {}", provider_status(&config.providers.openai));
// etc.
```

**Step 2: Commit**

```bash
git add src/cli/status.rs
git commit -m "refactor: extract status/auth commands into cli/status.rs"
```

---

## Task 9: Create cli/config.rs

**Files:**
- Create: `src/cli/config.rs`

**Step 1: Move config command**

Move `cmd_config()` (lines 203-247) from `main.rs`.

Takes `ConfigAction` from `super::ConfigAction`.

**Step 2: Commit**

```bash
git add src/cli/config.rs
git commit -m "refactor: extract config command into cli/config.rs"
```

---

## Task 10: Slim down main.rs and wire everything together

**Files:**
- Modify: `src/main.rs` (replace 2,212 lines with ~15 lines)
- Modify: `src/cli/mod.rs` (ensure all pub mods and dispatch work)

**Step 1: Replace main.rs**

```rust
mod cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cli::run().await
}
```

**Step 2: Run `cargo build`**

Fix any compilation errors. Common issues will be:
- Missing imports in submodules
- Visibility (`pub(crate)` vs `pub`)
- Cross-module references (e.g., gateway needs `heartbeat::heartbeat_file_path`)

**Step 3: Run `cargo test`**

Ensure all 589 tests still pass. This is a pure refactor — zero behavior change.

```bash
cargo test
```

**Step 4: Run `cargo clippy -- -D warnings`**

Fix any clippy warnings introduced by the move.

**Step 5: Commit**

```bash
git add src/main.rs src/cli/
git commit -m "refactor: slim main.rs to 15 lines, all CLI logic in src/cli/"
```

---

## Task 11: Clean up lib.rs re-exports (optional)

**Files:**
- Modify: `src/lib.rs`

**Step 1: Audit re-exports**

The current `lib.rs` has 57 lines of `pub use` statements re-exporting individual types. Review which are actually used by external consumers (tests, benchmarks) vs only used internally.

Keep re-exports that `tests/integration.rs` and `src/bin/benchmark.rs` use. Remove re-exports that are only used within `src/cli/` (those can use the full path `zeptoclaw::module::Type`).

**Step 2: Run tests to verify nothing breaks**

```bash
cargo test
```

**Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "refactor: trim lib.rs re-exports to public API surface"
```

---

## Summary

| Task | File | Lines moved | Description |
|------|------|-------------|-------------|
| 1 | cli/mod.rs | ~60 | CLI struct, enums, dispatch |
| 2 | cli/common.rs | ~280 | create_agent, helpers |
| 3 | cli/onboard.rs | ~530 | Onboarding wizard |
| 4 | cli/agent.rs | ~220 | Agent + stdin commands |
| 5 | cli/gateway.rs | ~300 | Gateway command |
| 6 | cli/heartbeat.rs | ~70 | Heartbeat command |
| 7 | cli/skills.rs | ~75 | Skills command |
| 8 | cli/status.rs | ~330 | Status + auth commands |
| 9 | cli/config.rs | ~50 | Config check command |
| 10 | main.rs | -2200 | Wire it all together |
| 11 | lib.rs | ~-30 | Trim re-exports (optional) |

**After:** `main.rs` ~15 lines. Largest file in `cli/` is `onboard.rs` at ~530 lines (acceptable — it's a linear wizard). No file exceeds 600 lines.

**Verification:** `cargo test && cargo clippy -- -D warnings && cargo build --release`
