# 003 — EdgeCrab Current State

| Existing | File |
|----------|------|
| LSP crate exists | `crates/edgecrab-lsp/` |
| `file_write` tool | `crates/edgecrab-tools/src/tools/file_write.rs` |
| `file_patch` tool | `crates/edgecrab-tools/src/tools/file_patch.rs` |

## What Is Missing

1. **No call site.** `edgecrab-lsp` is not invoked from `file_write` or
   `file_patch`.
2. **No language router.** No `extension → server` registry. The LSP crate
   may have a generic client but no auto-spawn-by-extension wiring.
3. **No diagnostic embedding in tool result.** Tool returns plain `{ok:true}`;
   need to extend the schema to include a `diagnostics` array.
4. **No graceful timeout.** If `rust-analyzer` hasn't indexed yet, the tool
   should still return after N ms with `diagnostics: []` and a soft warning.

## Honest Assessment

This is the lowest-hanging Tier-S fruit *if* `edgecrab-lsp` is reasonably
complete. If it's just a stub, this becomes a Tier-A bigger build. The
plan in `004-implementation-plan.md` assumes the crate provides a working
`pull_diagnostics(path)` future; if it does not, that work is in scope.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
