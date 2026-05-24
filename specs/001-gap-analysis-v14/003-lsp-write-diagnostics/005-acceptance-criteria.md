# 003 — Acceptance Criteria

## Functional

- [x] After `file_write` to a `.rs` file containing a typo, the tool
      result includes a `diagnostics` array with the rust-analyzer error.
      _(Proven with mock LSP server in `write_file_result_includes_lsp_diagnostics`; real rust-analyzer is config-dependent.)_
- [x] Model sees the diagnostic in the tool result and can self-correct
      in the next tool call. (`diagnostics` + `lsp_diagnostics` on success JSON.)
- [x] Read-only operations (`file_read`, `file_search`) are unaffected.
- [x] When `lsp.enabled: false`, tools behave exactly as today.
- [x] When the LSP server is not installed, tools succeed with
      `diagnostics: []` and a one-time warning logged.

## Performance

- [x] LSP server is spawned at most once per workspace root per session.
      _(Existing `edgecrab-lsp` manager; unchanged.)_
- [x] Diagnostic fetch returns within `lsp.timeout_ms` (default 1500).
- [x] No regression in `file_write` latency when LSP is disabled.
      _(Early return in `attach_post_write_diagnostics`.)_

## Languages Supported (out of the box)

- [x] Rust via `rust-analyzer`. _(User `lsp.servers` config — same as pre-existing LSP tools.)_
- [x] Python via `pylsp` or `pyright` (configurable).
- [x] TypeScript/JavaScript via `tsserver`.

## Code Quality

- [x] `cargo clippy --workspace -- -D warnings`.
- [x] `cargo test -p edgecrab-tools lsp_gate::` includes a mock `LspGate`
      implementation and a test verifying diagnostics appear in tool result.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
- Proof: [proof/implementation-proof.md](proof/implementation-proof.md)
