# 017 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Output redaction | redaction pipeline applied to assistant final output |
| `ToolError` enum | `crates/edgecrab-types/src/` |
| Tool dispatch | `crates/edgecrab-tools/src/registry.rs` `ToolRegistry::dispatch` |
| Conversation loop | `crates/edgecrab-core/src/conversation.rs` (pushes tool result back) |

## What Is Missing

1. No sanitiser applied between `ToolHandler::execute` Err and the
   message pushed back into the LLM context.
2. No central regex catalog for secret patterns.
3. No per-tool opt-out mechanism.

## Honest Assessment

The fastest, lowest-risk security win in the entire gap analysis.
Add one module, one call site, ten regex patterns. Significant
data-hygiene improvement, hours of implementation.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
