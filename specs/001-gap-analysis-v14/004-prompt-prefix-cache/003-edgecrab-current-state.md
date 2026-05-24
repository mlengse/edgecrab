# 004 — EdgeCrab Current State

| Existing | File |
|----------|------|
| System prompt builder | `crates/edgecrab-core/src/prompt_builder.rs` |
| Per-session prompt cache | `SessionState::cached_system_prompt` |
| Anthropic provider | (provider crate; check `model_router.rs`) |

## What Is Missing

1. **No prefix/suffix split.** The whole system prompt is one string —
   stable identity bits + volatile date stamp live together.
2. **No `cache_control: ttl: "1h"` attachment** in the Anthropic adapter.
3. **No cross-process cache reuse signalling.** Even if the prefix were
   stable, EdgeCrab doesn't currently emit cache breakpoint markers.
4. **No per-turn cost telemetry of `cache_read_input_tokens`** to *prove*
   the cache is working.

## Honest Assessment

The AGENTS.md doc warns "DO NOT rebuild the system prompt mid-conversation"
— so the in-session cache is preserved. But cross-session cache is left on
the table. For heavy users this is hundreds of dollars per month wasted.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
