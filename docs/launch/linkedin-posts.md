# ZeptoClaw LinkedIn Launch Sequence

**Brand:** Aisar Labs
**Product:** ZeptoClaw
**License:** Apache 2.0
**GitHub:** github.com/qhkm/zeptoclaw
**Site:** zeptoclaw.pages.dev

---

## Schedule

| Day | Post | Hook |
|---|---|---|
| Monday wk1 | Problem statement | "Why can't you deploy AI in enterprise?" |
| Wednesday wk1 | Security audit | "We audited 5 frameworks. All failed." |
| Friday wk1 | Multi-tenant gap | "50 clients, 50 servers? That's broken." |
| Monday wk2 | Dependency problem | "CISO killed the project. 1,200 packages." |
| Wednesday wk2 | Launch tease | "Four problems. One framework." |
| Friday wk2 | Launch | Drop the link |

**Best posting time:** 8:30 AM MYT (GMT+8)

---

## Post 1: The Problem Statement (Monday wk1)

At Aisar Labs, we're solving one problem:

Why can't you deploy an AI assistant in an enterprise without your security team killing the project?

We looked at every open-source AI agent framework. They all fail the same enterprise checklist:

❌ No container isolation — agent runs with server privileges
❌ No multi-tenancy — one agent per server, can't scale
❌ No audit trail — no idea what the agent actually did
❌ 500MB+ install — IT won't approve the dependency tree
❌ No SSRF protection — agent can probe your internal network
❌ No data isolation — tenant A can access tenant B's files

These frameworks were built for demos. Not for production. Not for compliance. Not for the enterprise procurement gauntlet.

We're building what's missing.

More soon.

#AI #Enterprise #CyberSecurity #AisarLabs

---

## Post 2: The Security Gap (Wednesday wk1)

We ran a security audit on 5 popular AI agent frameworks at Aisar Labs.

Every single one failed.

Here's what we found:

1. Shell commands execute as the host user. No sandbox. No container. Full disk access.

2. Zero SSRF protection on web fetch tools. The agent can probe 10.0.0.0/8 and 169.254.169.254. That's your internal network and cloud metadata.

3. No path traversal detection. The agent can read /etc/passwd through symlinks.

4. No command blocklist. rm -rf /? The framework will run it.

5. No rate limiting on tool calls. A hallucinating agent can spawn infinite processes.

These aren't edge cases. These are the first 5 things any pentester would try.

Enterprise AI adoption is blocked not because the models aren't good enough.

It's blocked because the frameworks aren't safe enough.

Aisar Labs is building the framework your security team will actually approve.

#CyberSecurity #AI #Enterprise #Pentesting #AisarLabs

---

## Post 3: The Multi-Tenant Gap (Friday wk1)

A telco asked us: "Can we give each of our 50 business clients their own AI assistant?"

We looked at what's available.

Every framework assumes: one agent, one server.

Want 50 clients? Spin up 50 servers. At $20/month each, that's $1,000/month before a single message is sent.

But here's the thing — an idle AI agent uses almost nothing. It's just waiting for a Telegram message.

50 agents should fit on one $5 VPS.

The real problem isn't compute. It's isolation.

→ Client A's agent must never see Client B's data
→ Client A's shell commands must stay in Client A's container
→ If Client A's agent goes rogue, Client B doesn't notice
→ Each client needs their own API keys, channels, workspace

Multi-tenancy is a solved problem for databases. For web apps. For SaaS platforms.

Nobody's solved it for AI agent frameworks.

That's exactly what we're building at Aisar Labs.

#AI #SaaS #MultiTenant #Enterprise #AisarLabs

---

## Post 4: The Dependency Problem (Monday wk2)

An enterprise security team rejected an AI agent deployment.

Not because of the AI. Because of the dependencies.

The framework pulled in 1,200 packages. 847 transitive dependencies. 500MB installed.

Their CISO asked one question: "Can you vouch for every package in this tree?"

The answer was no. Project killed.

This is happening everywhere. Supply chain attacks are real. SolarWinds. Log4j. node-ipc.

At Aisar Labs we asked: what if the entire AI agent framework had 12 dependencies?

Not 1,200. Twelve.

Same features. Shell execution, file ops, web search, cron scheduling, multi-channel messaging.

But a dependency tree your security team can actually review in an afternoon.

That's what we're building. Announcing soon.

#SupplyChain #CyberSecurity #Enterprise #AI #AisarLabs

---

## Post 5: The Launch Tease (Wednesday wk2)

Four problems. One framework.

❌ 500MB installs → We got it to 5MB
❌ No container isolation → Every command is sandboxed
❌ No multi-tenancy → 100 agents on one VPS
❌ 1,200 dependencies → We use 12

Written in Rust. 589 tests. Zero runtime crashes. Apache 2.0. Open source.

Aisar Labs is launching this Friday.

#AI #OpenSource #Rust #Enterprise #AisarLabs

---

## Post 6: Launch Day (Friday wk2)

Today Aisar Labs is open-sourcing ZeptoClaw.

An enterprise-grade AI assistant framework in Rust.

We built it because every AI agent framework we evaluated failed the enterprise security checklist.

So we built one that passes.

→ 5MB binary (not 500MB of dependencies)
→ 12 crate dependencies (not 1,200 packages)
→ Container isolation per request (Docker / Apple Container)
→ Multi-tenant — 100 agents on one $5 VPS
→ 7 layers of security (SSRF, path traversal, shell blocklist)
→ 16+ tools (shell, web, memory, cron, WhatsApp, Google Sheets)
→ 7 LLM providers (Claude, GPT-4, Gemini, Groq + more)
→ Multi-channel (Telegram, Slack, WhatsApp, CLI)
→ 589 tests. Zero runtime crashes.

Apache 2.0. Use it. Fork it. Deploy it.

→ GitHub: github.com/qhkm/zeptoclaw
→ Site: zeptoclaw.pages.dev

Built in Malaysia. Ready for enterprise.

#OpenSource #AI #Rust #Enterprise #MadeinMalaysia #AisarLabs
