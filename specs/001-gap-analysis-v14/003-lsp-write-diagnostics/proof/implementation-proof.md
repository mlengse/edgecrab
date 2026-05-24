# 003 — LSP Write Diagnostics — Implementation Proof

**Branch:** `feat/lsp-write-diagnostics`  
**Date:** 2026-05-24  
**Status:** Implemented

## What Was Built

| Component | Location |
|-----------|----------|
| `LspGate` trait + attach helper | `crates/edgecrab-tools/src/lsp_gate.rs` |
| `EdgecrabLspGate` (pull / push diagnostics) | `crates/edgecrab-lsp/src/gate.rs` |
| `ToolContext.lsp_gate` injection | `crates/edgecrab-tools/src/registry.rs` |
| Conversation wiring | `crates/edgecrab-core/src/conversation.rs` (`post_write_lsp_gate`, parallel dispatch clone fix) |
| Tool hooks | `file_write.rs`, `file_patch.rs` (`patch`, `apply_patch`) |
| Config | `lsp.enabled`, `lsp.timeout_ms` → `AppConfigRef.lsp_post_write_timeout_ms` |

### Behaviour

1. After a **successful** `write_file`, `patch`, or `apply_patch`, tools call `attach_post_write_diagnostics`.
2. When `lsp.enabled` is true and `ToolContext.lsp_gate` is set, the gate opens/syncs the document and pulls diagnostics (LSP 3.17 `textDocument/diagnostic` when supported, else push-cache wait).
3. Tool JSON gains:
   - `diagnostics`: `[{ severity, line, message, code? }, …]` (structured, errors + warnings only)
   - `lsp_diagnostics`: Hermes-compatible prose block for models that scan text fields
4. **Graceful degradation:** LSP disabled → no extra fields; no server for extension → `diagnostics: []` + one-time `tracing::warn!`; timeout → empty array; non-local terminal backend → skipped (matches Hermes `_lsp_local_only`).
5. **Timeout:** outer `tokio::time::timeout` uses `lsp.timeout_ms` (default **1500 ms**).

## Tests Run

```bash
cargo test -p edgecrab-tools --lib lsp_gate::
cargo test -p edgecrab-lsp --test lsp_tools_integration write_file_result_includes_lsp_diagnostics
cargo test -p edgecrab-lsp --test lsp_tools_integration
cargo test -p edgecrab-tools --lib file_write::
cargo clippy --workspace -- -D warnings
```

| Test | Result |
|------|--------|
| `lsp_gate::attach_injects_diagnostics_array` | pass (mock gate) |
| `lsp_gate::attach_noop_when_lsp_disabled` | pass |
| `write_file_result_includes_lsp_diagnostics` | pass (mock LSP server, real `write_file` + `EdgecrabLspGate`) |
| `lsp_tools_integration` (3 tests) | pass |
| `file_write::` unit suite (24 tests) | pass |

## Brutally Honest Hermes Parity Assessment

**Reference:** `hermes-agent/tools/file_operations.py` (`_maybe_lsp_diagnostics`), `WriteResult.lsp_diagnostics`, `tests/agent/lsp/test_diagnostics_field.py`.

| Capability | Hermes | EdgeCrab (this PR) | Verdict |
|------------|--------|-------------------|---------|
| Post-write LSP on `write_file` | yes | yes | **Parity** |
| Post-write on `patch` / V4A | yes (`PatchResult.lsp_diagnostics`) | yes (`patch`, `apply_patch`) | **Parity** |
| `lsp_diagnostics` text field for model | yes (`<diagnostics>` XML via reporter) | yes (simpler line-oriented block + structured `diagnostics`) | **Near parity** (format differs, signal equivalent) |
| Skip when LSP off / unavailable | yes | yes | **Parity** |
| Skip non-local backends | yes | yes (`BackendKind::Local`) | **Parity** |
| Timeout-bound fetch | yes (service-internal) | yes (`lsp.timeout_ms`, default 1500) | **Parity** |
| **Delta diagnostics** (only errors *introduced* by edit; `line_shift` remap) | yes | **no** — returns current document diagnostics after refresh | **Gap** — may surface pre-existing issues below edit; acceptable for v1, document as follow-up |
| Baseline snapshot before write | yes (`snapshot_baseline`) | not wired on write path | **Gap** (feeds delta in Hermes) |
| Truncation of huge diagnostic blocks | yes (`truncate`) | not yet | **Minor gap** |
| rust-analyzer / pyright / tsserver OOTB | via user LSP config | via `lsp.servers` in `config.yaml` | **Parity** (config-driven) |

**Rust-specific notes:** EdgeCrab correctly avoids a tools→lsp crate cycle via `LspGate` trait injection; Hermes uses a Python singleton `get_service()`. Parallel tool dispatch required cloning `lsp_gate` before `spawn` (lifetime fix in `conversation.rs`).

**Overall:** For a Rust agent, this **matches or exceeds** Hermes on the core user-visible contract (semantic errors in the same tool result turn). It **does not yet match** Hermes v0.13 **delta** semantics; that is the main honest gap. Composes cleanly with **002 file-mutation verifier** (landed paths vs LSP errors are orthogonal signals).

## Edge Cases & Mitigations

| Edge case | Mitigation |
|-----------|------------|
| LSP disabled | `attach_post_write_diagnostics` returns immediately |
| No gate in context (schema tests, gateway pre-analysis) | `lsp_gate: None` — no diagnostics fields |
| No server for file extension | empty `diagnostics`, one-time warn |
| Pull timeout | empty array, `tracing::debug!` |
| LSP server crash mid-request | caught in gate → empty diagnostics, write still succeeds |
| Parallel `write_file` calls | each dispatch gets cloned `Arc<dyn LspGate>` |
| Unicode paths | path resolution via existing `jail_read_path` / canonicalize |
| Huge files | existing `lsp_file_size_limit_bytes` on open/sync |

## Follow-ups (not in scope)

1. Delta diagnostics + `line_shift` remapping (Hermes `agent.lsp.range_shift`).
2. Optional truncation cap on `lsp_diagnostics` string length.
3. Live Copilot E2E with real `rust-analyzer` (mock server proves wiring; analyzer E2E is environment-dependent).
