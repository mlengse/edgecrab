# 008 — OpenAI-Compatible Local Proxy (`edgecrab proxy`)

**Tier:** A | **Impact:** 5 | **Value-per-Effort:** 3 | **Risk:** 3
**Primitive moved:** Cost per useful turn (huge), Reach (massive)

## Why It Matters (First Principles)

There are two LLM ecosystems on Earth:

1. **OpenAI-shaped HTTP API** — every third-party tool (Codex CLI, Aider,
   Cline, Continue, Cursor, LiteLLM, LangChain, etc.) speaks this dialect.
2. **OAuth-gated provider APIs** — Anthropic Claude Pro subscription,
   ChatGPT Pro subscription, xAI SuperGrok, GitHub Copilot — accessible
   only through their own OAuth flow inside their own clients.

A user who pays $200/month for Claude Pro can use it only inside Claude
Desktop / Claude Code. If EdgeCrab exposes that OAuth-gated provider as a
**local OpenAI-compatible endpoint** (`http://127.0.0.1:11434/v1`), the
user can now wire their $200 subscription into Aider, Cline, the OpenAI
Python SDK, LiteLLM — anything. This is the v0.14 "OpenAI-compat
local proxy" feature and it is **transformative for cost**: it monetises
a subscription the user already pays for.

## The Gap

EdgeCrab has zero proxy surface. The only way to use EdgeCrab is via the
TUI, ACP adapter, or gateway. The model catalog supports many providers
internally, but none of them are exposed externally as an OpenAI server.

## What EdgeCrab Gets Wrong Today

A power user with paid subscriptions to Claude Pro + ChatGPT Pro + xAI
SuperGrok can use **none** of them inside their preferred coding tools.
They are forced to either:
- Use the official client (locked to one tool),
- Pay extra for API access (defeats the subscription's purpose),
- Glue together third-party gray-market proxies (security nightmare).

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Depends on: [024-oauth-providers/](../024-oauth-providers/) (provides the OAuth backends to expose)
- Composes with: [010-mcp-sse-oauth-parallel/](../010-mcp-sse-oauth-parallel/) (proxy can also expose MCP tools as OpenAI functions)
