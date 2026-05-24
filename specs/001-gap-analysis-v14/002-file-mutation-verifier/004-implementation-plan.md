# 002 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────┐
   │                    edgecrab-core                             │
   │                                                              │
   │   ┌──────────────┐  records  ┌──────────────────────────┐    │
   │   │ file_write   │──────────►│  MutationBuffer          │    │
   │   │ file_patch   │           │   (Arc<Mutex<Vec<Rec>>>) │    │
   │   │ delete_file  │           │   - push(record)         │    │
   │   └──────────────┘           │   - drain() -> Vec       │    │
   │                              └──────────┬───────────────┘    │
   │                                         │ end of turn        │
   │                                         ▼                    │
   │                              ┌──────────────────────────┐    │
   │                              │ render_mutation_footer   │    │
   │                              │   -> String              │    │
   │                              └──────┬─────────────┬─────┘    │
   │                                     │             │          │
   │                            stream   ▼             ▼ history  │
   │                                   user        next turn      │
   └──────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **New module** | `crates/edgecrab-core/src/mutations/mod.rs` — `MutationRecord`, `MutationKind`, `MutationBuffer`, `render_mutation_footer` |
| **Tool integration** | `file_write.rs`, `file_patch.rs`, `file_search.rs` (delete path) — call `ctx.mutation_buffer.push(...)` on success |
| **ToolContext** | `crates/edgecrab-tools/src/registry.rs` — add `pub mutation_buffer: Arc<Mutex<MutationBuffer>>` |
| **Loop integration** | `crates/edgecrab-core/src/conversation.rs` — at end of `execute_loop`, drain buffer, render footer, emit as `StreamEvent::Footer`, and push `Message::system_note(footer)` into messages for next turn |
| **CLI render** | `crates/edgecrab-cli/src/app.rs` — handle new `StreamEvent::Footer` variant |
| **Gateway render** | `crates/edgecrab-gateway/src/stream_consumer.rs` — same |

## DRY / SOLID Notes

- **SRP:** `MutationBuffer` only records; `render_mutation_footer()` is a
  pure function in `mutations/mod.rs`.
- **OCP:** new mutation kinds (e.g. `Rename`) add a `MutationKind` variant
  and a glyph mapping — no caller changes.
- **DRY:** the same `render_mutation_footer()` powers TTY, gateway,
  and next-turn injection.
- **Cache safety:** the footer becomes a *user-role* system note appended
  to history, NOT a system prompt mutation — see [../004-prompt-prefix-cache/](../004-prompt-prefix-cache/).

## Diff Counting

For `file_write` we compare bytes before/after (treat new file as +N, 0).
For `file_patch` we use the patch hunks directly. For `delete_file` we
record −line_count_at_delete_time.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- LSP-related: [../003-lsp-write-diagnostics/004-implementation-plan.md](../003-lsp-write-diagnostics/004-implementation-plan.md)
