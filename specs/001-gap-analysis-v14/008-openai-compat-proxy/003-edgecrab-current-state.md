# 008 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Provider trait | `crates/edgecrab-core/src/` (`LLMProvider`) |
| Model router | `crates/edgecrab-core/src/model_router.rs` |
| Anthropic adapter | (provider module within core) |
| Gateway API server platform | `crates/edgecrab-gateway/src/platforms/api_server.rs` |
| CLI subcommands | `crates/edgecrab-cli/src/main.rs` |

## What Is Missing

1. **No `edgecrab proxy` subcommand.** Need a new CLI subcommand.
2. **No OpenAI-shape HTTP server.** Gateway's `api_server` platform is
   EdgeCrab-specific JSON-RPC, not OpenAI-compatible.
3. **No request translation layer.** Even with all the right providers,
   nothing translates OpenAI `tools` schema → Anthropic `tools` schema and
   back.
4. **No SSE re-emitter.** Streaming token translation absent.
5. **No local Bearer token store + middleware.**
6. **No OAuth providers** (depends on [024-oauth-providers/](../024-oauth-providers/)).
7. **No model alias map** distinguishing "Claude API key" from
   "Claude Pro OAuth subscription."

## Honest Assessment

This is the single highest-leverage **distribution** feature in the entire
gap analysis. It transforms EdgeCrab from "another CLI agent" into
"the bridge between paid subscriptions and the OpenAI tool ecosystem."
But it has hard dependencies — chiefly OAuth providers (folder 024).
Sequence carefully.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
- Hard dependency: [../024-oauth-providers/](../024-oauth-providers/)
