# Comparison: ZeptoClaw vs NanoBot vs NanoClaw

A detailed comparison of three lightweight AI assistant frameworks.

## Overview

| Feature | **ZeptoClaw** | **NanoBot** | **NanoClaw** |
|---------|---------------|-------------|--------------|
| **Language** | Rust | Python | TypeScript |
| **Lines of Code** | ~24,000 | ~8,500 (~3,500 core) | ~5,000 |
| **Philosophy** | Lightweight, secure, self-hosted | Ultra-lightweight, research-ready | Minimal, fork & customize |

---

## Channels Supported

| Channel | **ZeptoClaw** | **NanoBot** | **NanoClaw** |
|---------|:-------------:|:-----------:|:------------:|
| CLI | Yes | Yes | No |
| Telegram | Yes | Yes | Via skill |
| WhatsApp | Yes (Cloud API tool) | Yes | Yes (primary) |
| Slack | Yes (outbound) | Yes | Via skill |
| Discord | No | Yes | Via skill |
| Feishu | No | Yes | No |
| Email | No | Yes | No |

---

## Tools & Capabilities

| Tool | **ZeptoClaw** | **NanoBot** | **NanoClaw** |
|------|:-------------:|:-----------:|:------------:|
| Shell execution | Yes (containerized) | Yes | Yes (in container) |
| File read/write/edit | Yes | Yes | Yes |
| Web search | Yes (Brave) | Yes (Brave) | Yes |
| Web fetch | Yes (SSRF-hardened) | Yes | Yes |
| WhatsApp messaging | Yes (Cloud API) | Yes | Yes |
| Google Sheets | Yes | No | No |
| Scheduled tasks (cron) | Yes | Yes | Yes |
| Background spawn | Yes | Yes | No |
| Proactive messaging | Yes | No | No |
| Voice transcription | No | Yes (Groq Whisper) | No |
| Agent Swarms | No | No | Yes |
| Skills system | Yes (markdown) | Yes (bundled) | Yes (transform forks) |
| Heartbeat service | Yes | No | No |

---

## Security & Isolation

| Feature | **ZeptoClaw** | **NanoBot** | **NanoClaw** |
|---------|---------------|-------------|--------------|
| **Shell sandboxing** | Container runtime (Docker/Apple) | Workspace restriction | Container isolation |
| **Gateway isolation** | Containerized agent per request | None | Container per group |
| **Shell blocklist** | Regex patterns | No | Container isolation |
| **Path traversal** | Symlink escape detection | No | Container mounts |
| **SSRF prevention** | DNS pre-check, redirect validation | No | No |
| **Credential protection** | Blocked patterns for SSH/AWS | No | Isolated filesystem |
| **Mount validation** | Allowlist + blocked patterns | No | Mount policy |
| **Rate limiting** | Cron caps, spawn depth limit | No | No |
| **Input validation** | URL path, spreadsheet ID checks | No | No |

---

## LLM Providers

| Provider | **ZeptoClaw** | **NanoBot** | **NanoClaw** |
|----------|:-------------:|:-----------:|:------------:|
| Anthropic/Claude | Yes | Yes | Yes (via SDK) |
| OpenAI | Yes | Yes | No |
| OpenRouter | Config only | Yes | No |
| Local (vLLM) | Via OpenAI endpoint | Yes | No |
| DeepSeek | Via OpenAI endpoint | Yes | No |
| Groq | Via OpenAI endpoint | Yes | No |
| Gemini | Config only | Yes | No |

---

## Architecture

| Aspect | **ZeptoClaw** | **NanoBot** | **NanoClaw** |
|--------|---------------|-------------|--------------|
| **Runtime** | Tokio async | Python asyncio | Node.js |
| **Message Bus** | Internal pub/sub | Simple routing | Polling loop + queue |
| **Agent Execution** | In-process or containerized | In-process | Container per group |
| **Tool System** | Trait-based registry (13 tools) | Skills + tools | Claude Agent SDK |
| **Session Storage** | JSON files | JSONL + markdown | SQLite |
| **Memory** | Markdown workspace files | Two-layer (facts + log) | Per-group CLAUDE.md |

---

## Performance

| Metric | **ZeptoClaw** | **NanoBot** | **NanoClaw** |
|--------|---------------|-------------|--------------|
| **Binary Size** | ~4MB | N/A (Python) | N/A (Node.js) |
| **Startup Time** | ~50ms | Medium | Medium |
| **Memory Usage** | ~6MB RSS | Medium | Higher (containers) |
| **Test Suite** | 498 tests | - | - |

---

## Key Differentiators

### ZeptoClaw (Rust)
**Strengths**: Security-hardened (SSRF, path traversal, shell blocklist, mount validation), native binary performance, 13 integrated tools, heartbeat service, skills system, containerized gateway with concurrency control.

**Trade-offs**: Fewer channels than NanoBot, no voice transcription, no agent swarms.

### NanoBot (Python)
**Strengths**: Most LLM providers (10+), most channels (9+), voice transcription, easy to extend via Python ecosystem.

**Trade-offs**: No container isolation, no security hardening, larger dependency footprint.

### NanoClaw (TypeScript)
**Strengths**: True container isolation per group, agent swarms, SQLite reliability, smallest codebase.

**Trade-offs**: WhatsApp-only (others via skills), Claude-only, requires container runtime.

---

## Decision Matrix

| If you want... | Choose |
|----------------|--------|
| Best security + performance | ZeptoClaw |
| Most integrations + Python | NanoBot |
| Container isolation + fork customization | NanoClaw |
| Native binary distribution | ZeptoClaw |
| Voice messages | NanoBot |
| Agent teams/swarms | NanoClaw |
| E-commerce tools (WhatsApp + Sheets) | ZeptoClaw |
| Scheduled background tasks | ZeptoClaw or NanoBot |
