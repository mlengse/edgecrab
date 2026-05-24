# 003 — Implementation Plan

## Architecture (ASCII)

```
   ┌────────────────────────────────────────────────────────────────┐
   │                      edgecrab-tools                            │
   │                                                                │
   │   ┌──────────────┐ success ┌──────────────────────────────┐    │
   │   │ file_write   │────────►│  LspGate (trait)             │    │
   │   │ file_patch   │         │  pull_diagnostics(path,      │    │
   │   └──────────────┘         │                   timeout)   │    │
   │                            └──────┬───────────────────────┘    │
   │                                   │                            │
   └───────────────────────────────────┼────────────────────────────┘
                                       │ impl
                                       ▼
   ┌────────────────────────────────────────────────────────────────┐
   │                     edgecrab-lsp                               │
   │                                                                │
   │   ┌──────────────────────┐    ┌────────────────────────────┐   │
   │   │ LanguageRouter       │───►│ServerPool                  │   │
   │   │  ext → server name   │    │  rust-analyzer (long-lived)│   │
   │   │  ".rs" → rust-an.    │    │  pylsp                     │   │
   │   │  ".py" → pylsp       │    │  tsserver                  │   │
   │   └──────────────────────┘    └────────────────────────────┘   │
   └────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **New trait** | `crates/edgecrab-tools/src/lsp_gate.rs` — `LspGate { async fn pull_diagnostics(&self, path: &Path, timeout: Duration) -> Vec<Diagnostic> }` |
| **Wiring** | `crates/edgecrab-tools/src/registry.rs` — `ToolContext` gains `lsp_gate: Option<Arc<dyn LspGate>>` |
| **Default impl** | `crates/edgecrab-lsp/src/gate.rs` — `LspGateImpl { router, pool }` |
| **Language router** | `crates/edgecrab-lsp/src/router.rs` — extension → server config (configurable via `~/.edgecrab/config.yaml` `lsp.servers`) |
| **Tool integration** | `file_write.rs` and `file_patch.rs` — after successful write, call `if let Some(gate) = &ctx.lsp_gate { gate.pull_diagnostics(...).await }` with a 1.5 s default timeout |
| **Tool result schema** | extend success JSON: `{ ok: true, path, diagnostics: [...] }` |
| **Config** | `lsp.enabled: true`, `lsp.timeout_ms: 1500`, `lsp.servers: { rust: ["rust-analyzer"], py: ["pylsp"] }` |

## DRY / SOLID Notes

- **SRP:** `LspGate` only fetches diagnostics for a path; spawning servers
  is `ServerPool`'s job; mapping extensions is `LanguageRouter`'s.
- **DIP:** `edgecrab-tools` depends on the `LspGate` *trait*, not on
  `edgecrab-lsp` directly. The trait lives in `edgecrab-tools` to keep the
  dependency arrow correct.
- **OCP:** new languages add a router entry, no code changes.
- **Graceful degradation:** when `lsp_gate` is `None` or `timeout` elapses,
  tools return `diagnostics: []` and a debug log line — never block.

## Performance

- LSP servers are **long-lived per workspace root**, not spawned per write.
- Diagnostics fetch uses `textDocument/diagnostic` (pull model, LSP 3.17+);
  fall back to a 100 ms wait on `publishDiagnostics` if the server only
  supports push.
- Total added latency: 50–400 ms typical, capped at 1500 ms.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Composes with: [../002-file-mutation-verifier/](../002-file-mutation-verifier/)
