# 003 — Acceptance Criteria

## Functional

- [ ] After `file_write` to a `.rs` file containing a typo, the tool
      result includes a `diagnostics` array with the rust-analyzer error.
- [ ] Model sees the diagnostic in the tool result and can self-correct
      in the next tool call.
- [ ] Read-only operations (`file_read`, `file_search`) are unaffected.
- [ ] When `lsp.enabled: false`, tools behave exactly as today.
- [ ] When the LSP server is not installed, tools succeed with
      `diagnostics: []` and a one-time warning logged.

## Performance

- [ ] LSP server is spawned at most once per workspace root per session.
- [ ] Diagnostic fetch returns within `lsp.timeout_ms` (default 1500).
- [ ] No regression in `file_write` latency when LSP is disabled.

## Languages Supported (out of the box)

- [ ] Rust via `rust-analyzer`.
- [ ] Python via `pylsp` or `pyright` (configurable).
- [ ] TypeScript/JavaScript via `tsserver`.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] `cargo test -p edgecrab-tools lsp_gate::` includes a mock `LspGate`
      implementation and a test verifying diagnostics appear in tool result.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
