# ZeptoClaw Launch Announcement

2026-02-13 🎉 **ZeptoClaw Launched!** Built to bring AI Agents to the absolute edge with zero-copy efficiency and fearless concurrency. 🦀 Rust all the way down!

## ✨ Features

🦀 **Memory Safe**: Zero undefined behavior — Rust's ownership model guarantees memory safety without garbage collection overhead.

🔒 **Container Isolation**: Selectable runtime security — Native, Docker, or Apple Container for defense-in-depth shell execution.

⚡️ **Async Everything**: Tokio-powered async runtime — Handle thousands of concurrent sessions without blocking.

🌍 **True Portability**: Single static binary — Cross-compile to Linux, macOS, Windows, ARM, x86, even WASM.

🔧 **Type-Safe Config**: Serde-powered configuration — Compile-time guarantees for your config schema.

🤖 **Multi-Provider**: Claude, OpenAI, or bring your own — Swap LLM backends without code changes.

📡 **Channel Agnostic**: Telegram today, Discord/Slack tomorrow — Unified message bus architecture.

## Comparison

| | OpenClaw | NanoClaw | PicoClaw | **ZeptoClaw** |
|---|---|---|---|---|
| **Language** | TypeScript | Python | Go | **Rust** |
| **RAM** | >1GB | >100MB | <10MB | **<5MB** |
| **Startup** (0.8GHz) | >500s | >30s | <1s | **<0.5s** |
| **Binary Size** | N/A (Node) | N/A (Python) | ~15MB | **~8MB** |
| **Container Isolation** | ❌ | ✅ | ❌ | **✅** |
| **Memory Safety** | Runtime | Runtime | Runtime | **Compile-time** |
| **Async Model** | Event Loop | asyncio | Goroutines | **Tokio** |

## Why Zepto?

> Zepto (10⁻²¹) < Pico (10⁻¹²) < Nano (10⁻⁹)

ZeptoClaw is the smallest, fastest, safest member of the Claw family. When you need AI agents on truly constrained hardware — or just want the peace of mind that comes with Rust's guarantees — ZeptoClaw delivers.

## Quick Start

```bash
# Build
cargo build --release

# Configure
./target/release/zeptoclaw onboard

# Run
./target/release/zeptoclaw agent -m "Hello, ZeptoClaw!"
```

## Architecture Highlights

```
┌─────────────────────────────────────────────────────────────┐
│                      ZeptoClaw                               │
├─────────────────────────────────────────────────────────────┤
│  MessageBus (async channels)                                 │
│  ├── Telegram Channel                                        │
│  ├── CLI Channel                                             │
│  └── (Future: Discord, Slack, Matrix)                        │
├─────────────────────────────────────────────────────────────┤
│  AgentLoop (tool-calling state machine)                      │
│  ├── Session Management (SQLite/Memory)                      │
│  └── Context Building (sliding window)                       │
├─────────────────────────────────────────────────────────────┤
│  Providers (trait-based abstraction)                         │
│  ├── ClaudeProvider                                          │
│  └── OpenAIProvider                                          │
├─────────────────────────────────────────────────────────────┤
│  Tools (async_trait)                                         │
│  ├── ShellTool + ContainerRuntime                            │
│  ├── Filesystem Tools                                        │
│  └── (Extensible registry)                                   │
├─────────────────────────────────────────────────────────────┤
│  Runtime Isolation                                           │
│  ├── NativeRuntime (default)                                 │
│  ├── DockerRuntime (container isolation)                     │
│  └── AppleContainerRuntime (macOS 15+)                       │
└─────────────────────────────────────────────────────────────┘
```

## Built With

- **Rust 2021** — Modern, safe systems programming
- **Tokio** — Async runtime for reliable networking
- **Serde** — Zero-copy serialization
- **Reqwest** — Async HTTP client
- **GRDB/SQLite** — Lightweight persistence
- **Tracing** — Structured logging

## What's Next

- [ ] WebSocket channel support
- [ ] WASM runtime target
- [ ] Embedded Linux optimization
- [ ] MCP (Model Context Protocol) integration
- [ ] Skills/plugins system

---

*ZeptoClaw: Because sometimes the smallest claw has the strongest grip.* 🦀
