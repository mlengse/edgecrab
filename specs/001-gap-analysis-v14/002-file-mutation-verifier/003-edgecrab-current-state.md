# 002 — EdgeCrab Current State

| Existing | File |
|----------|------|
| `file_write` tool | `crates/edgecrab-tools/src/tools/file_write.rs` |
| `file_patch` tool | `crates/edgecrab-tools/src/tools/file_patch.rs` |
| `ToolContext` (shared per loop) | `crates/edgecrab-tools/src/registry.rs` |
| Streaming display | `crates/edgecrab-cli/src/app.rs` |

## What Is Missing

1. No `MutationBuffer` in `ToolContext`.
2. No interceptor between `file_write`/`file_patch` success and the user
   stream — the tool returns a JSON string and that's it.
3. No per-turn flush hook in `conversation.rs::execute_loop()`.
4. No injection of the mutation footer into the next-turn history.

## Honest Assessment

This is **embarrassingly easy** to ship and has outsized trust impact.
The only design question is "where does the buffer live?" — answer:
`ToolContext.mutation_buffer: Arc<Mutex<Vec<MutationRecord>>>`.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
