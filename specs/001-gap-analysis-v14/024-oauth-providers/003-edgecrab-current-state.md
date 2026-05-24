# 024 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Provider factory | `crates/edgecrab-core/src/model_router.rs` |
| Anthropic provider | API-key only |
| OpenAI provider | API-key only |
| Token storage | none (env vars + config only) |

## What Is Missing

1. OAuth flow (PKCE) infrastructure.
2. Token storage abstraction.
3. Device-code flow for Copilot.
4. Per-provider OAuth endpoint adapters (Claude/ChatGPT/Grok/Copilot).
5. Refresh-on-401 logic.
6. TUI integration: "Login with Claude Pro" flow.

## Honest Assessment

The biggest unlock and the most fiddly. Each provider's OAuth is
*slightly* different. The chat endpoints behave differently from
API endpoints. This is multi-week work for production-grade.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
