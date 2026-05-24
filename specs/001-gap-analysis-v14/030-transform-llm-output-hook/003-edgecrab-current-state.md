# 030 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Plugin system | `crates/edgecrab-plugins/` (WASM + Lua) |
| Output redaction | end-of-turn pipeline in `edgecrab-core` |
| Streaming | `StreamEvent` channel from agent.rs |

## What Is Missing

1. No `transform_output` hook in the plugin API.
2. No streamed-delta plugin invocation point.
3. No plugin ordering / pipeline concept.

## Honest Assessment

The existing redaction pipeline is essentially this hook — but it's
hardcoded as a single internal step. Generalise it into a plugin
pipeline; redaction becomes one built-in plugin in the chain.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
