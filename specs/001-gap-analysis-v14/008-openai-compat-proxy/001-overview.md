# 008 — OpenAI-Compatible Local Proxy (`edgecrab proxy`)

**Tier:** A | **Impact:** 5 | **Value-per-Effort:** 3 | **Risk:** 3
**Primitive moved:** Cost per useful turn (huge), Reach (massive)

## What This Feature Actually Is (v0.14 scope)

EdgeCrab ships a **local OpenAI-shaped HTTP server** so third-party clients (Aider,
Cline, OpenAI SDK, LiteLLM, etc.) can talk to `http://127.0.0.1:11434/v1` with a
**local proxy bearer** while the proxy attaches **real upstream credentials**.

Two modes:

| Mode | Purpose | Status |
|------|---------|--------|
| **A — Credential forwarder** | Hermes-style verbatim HTTP proxy with OAuth/static bearer swap (`nous`, `xai`, `hermes_auth`, `static`) | **Implemented** — Hermes reference parity for Nous + xAI |
| **B — Provider bridge** | OpenAI JSON ↔ `edgequake_llm::LLMProvider` (API keys from model catalog) | **Implemented** — no subscription OAuth translation |

**Not in 008:** Claude Pro / ChatGPT Pro / Copilot subscription unlock as OpenAI
endpoints. Those require [024-oauth-providers/](../024-oauth-providers/) and are
out of scope for the Hermes proxy reference (Hermes only ships `nous` + `xai`).

## Why It Matters

Users already pay for Nous Portal or xAI SuperGrok but are locked into each vendor’s
client. A localhost OpenAI-compat bridge lets the same subscription power Aider,
Cline, or any OpenAI SDK app — without buying duplicate API keys.

## The Gap (before EdgeCrab)

No external OpenAI server surface; only TUI, ACP, and gateway agent API.

## What EdgeCrab Delivers Today

- `edgecrab proxy` CLI + `/proxy` TUI wizard (ahead of Hermes CLI ergonomics)
- `crates/edgecrab-proxy/` — axum server, forwarder, Nous quarantine/allowlist, xAI pool rotation
- E2E: grok/xAI OAuth, Nous 401 retry, Nous quarantine, forward HTTP, CLI/slash commands
- Stricter local auth than Hermes (file token + timing-safe Bearer; Hermes accepts any client bearer)

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md) — ground-truth Hermes proxy
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md) — file map + tests
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Depends on: [024-oauth-providers/](../024-oauth-providers/) for future Claude/Copilot adapters
- Composes with: [010-mcp-sse-oauth-parallel/](../010-mcp-sse-oauth-parallel/)
