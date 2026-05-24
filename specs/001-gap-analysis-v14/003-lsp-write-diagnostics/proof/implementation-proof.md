# 003 — LSP Write Diagnostics — Implementation Proof

**Branch:** `feat/lsp-write-diagnostics`  
**Date:** 2026-05-24  
**Status:** Implemented — Hermes parity (delta + line shift + truncation)

## What Was Built

| Component | Location |
|-----------|----------|
| `LspGate` trait + attach helper | `crates/edgecrab-tools/src/lsp_gate.rs` |
| `LspWriteHook` (snapshot before / attach after) | `crates/edgecrab-tools/src/lsp_gate.rs` |
| `EdgecrabLspGate` | `crates/edgecrab-lsp/src/gate.rs` |
| Delta filter + diagnostic keys | `crates/edgecrab-lsp/src/delta.rs` |
| Line-shift remap | `crates/edgecrab-lsp/src/range_shift.rs` |
| Per-session delta baselines | `LspRuntime.delta_baselines` in `manager.rs` |
| Tool hooks | `file_write.rs`, `file_patch.rs` (`patch`, `apply_patch`) |
| Config | `lsp.enabled` (default **true**), `lsp.timeout_ms` (default **1500**) |

### Behaviour

1. **Before write:** `LspWriteHook::capture_before` reads pre-edit content and calls `snapshot_baseline` (Hermes `_snapshot_lsp_baseline`).
2. **After write:** `pull_diagnostics` with `LspEditContext { pre, post }` applies line-shift on baseline, subtracts unchanged diagnostics, rolls baseline forward.
3. Tool JSON: `diagnostics` array + Hermes-style `lsp_diagnostics` (`<diagnostics file="...">` block, max 20/file, 4000 chars total).
4. Graceful degradation unchanged (disabled LSP, no server, timeout, non-local backend).

## Tests Run

```bash
cargo test -p edgecrab-tools --lib lsp_gate::
cargo test -p edgecrab-lsp --lib range_shift delta
cargo test -p edgecrab-lsp --test lsp_tools_integration
cargo clippy --workspace -- -D warnings
```

| Test | Result |
|------|--------|
| `lsp_gate::*` (3) | pass |
| `range_shift::*` (3) | pass |
| `delta::filters_unchanged_diagnostics` | pass |
| `write_file_result_includes_lsp_diagnostics` | pass (mock LSP) |
| `write_file_delta_filters_preexisting_diagnostics` | pass (delta parity) |
| `lsp_tools_integration` (4 tests) | pass |

### Live certification (Copilot / `gpt-5-mini`)

```bash
cargo test -p edgecrab-core --test e2e_copilot e2e_lsp_write_diagnostics_with_copilot_gpt5_mini -- --include-ignored --nocapture
```

**2026-05-24 result:** passed in ~18s. Agent `write_file` on `src/lsp_e2e_broken.rs`; tool message contained LSP `diagnostics` / `lsp_diagnostics`.

## Brutally Honest Hermes Parity Assessment

**Reference:** `hermes-agent/tools/file_operations.py`, `agent/lsp/manager.py`, `agent/lsp/range_shift.py`, `agent/lsp/reporter.py`.

| Capability | Hermes | EdgeCrab | Verdict |
|------------|--------|----------|---------|
| Post-write LSP on `write_file` / `patch` | yes | yes | **Parity** |
| `lsp_diagnostics` + structured field | yes | yes (`diagnostics` + XML block) | **Parity** |
| `snapshot_baseline` before write | yes | yes (`LspWriteHook::capture_before`) | **Parity** |
| Delta (only introduced errors) | yes | yes (`delta.rs` + baseline map) | **Parity** |
| `line_shift` remap | yes | yes (`range_shift.rs` via `similar`) | **Parity** |
| Truncation (4000 chars) | yes | yes | **Parity** |
| Skip non-local backends | yes | yes | **Parity** |
| Syntax-tier gate (skip LSP if parse fails) | yes (lint tier) | no separate lint gate on write | **Minor gap** — EdgeCrab relies on LSP only; acceptable |
| Multi-file patch LSP aggregation | combined string | first created/modified file only | **Minor gap** for large V4A patches |

**Overall:** Core Hermes v0.13 contract is **matched**. Remaining gaps are edge cases (lint gating, multi-file patch rollup), not the primary write/patch path.

## Activation

LSP post-write diagnostics are **on by default** (`lsp.enabled: true` in `AppConfig`). Ensure language servers are installed (e.g. `rust-analyzer` on PATH) and listed under `lsp.servers` in `~/.edgecrab/config.yaml` (built-in defaults include rust, python, typescript).

## Edge Cases & Mitigations

| Edge case | Mitigation |
|-----------|------------|
| Rewrite same broken file | Delta filter returns empty `diagnostics` (integration test) |
| Line insert/delete below existing error | `build_line_shift` remaps baseline before diff |
| apply_patch multi-file | baseline snapshot per op; LSP attach on first touched file |
| Parallel writes | per-path baseline in `DashMap` |
