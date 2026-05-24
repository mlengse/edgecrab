# 003 — LSP Write Diagnostics — Implementation Proof

**Branch:** `feat/lsp-write-diagnostics`  
**Date:** 2026-05-24  
**Status:** Implemented — Hermes parity + `/lsp` toggle

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
| **Runtime toggle** | `/lsp on\|off\|status\|toggle` (CLI + gateway) |
| **Config persist** | `AppConfig::persist_lsp_enabled`, `EDGECRAB_LSP_ENABLED` |
| Config default | `lsp.enabled: true`, `lsp.timeout_ms: 1500` |

### Behaviour

1. **Before write:** `LspWriteHook::capture_before` reads pre-edit content and calls `snapshot_baseline`.
2. **After write:** delta + line-shift filtering; tool JSON gets `diagnostics` + Hermes `<diagnostics>` block (truncated).
3. **`/lsp off`:** disables LSP for the session and persists `lsp.enabled: false`; mutations skip post-write diagnostics.
4. **`/lsp on`:** re-enables and persists; `post_write_lsp_gate` injects `EdgecrabLspGate` on next turns.

## Tests Run

```bash
cargo test -p edgecrab-tools --lib lsp_gate
cargo test -p edgecrab-lsp --lib range_shift delta
cargo test -p edgecrab-lsp --test lsp_tools_integration
cargo test -p edgecrab-core --lib persist_lsp_enabled
cargo test -p edgecrab-cli --bin edgecrab dispatch_lsp_toggle
cargo clippy --workspace -- -D warnings
cargo test -p edgecrab-core --test e2e_copilot e2e_lsp_write_diagnostics_with_copilot_gpt5_mini -- --include-ignored
```

| Test | Result |
|------|--------|
| `lsp_gate::*` (4) | pass |
| `write_file_delta_filters_preexisting_diagnostics` | pass |
| `persist_lsp_enabled_round_trip_via_save_to` | pass |
| `dispatch_lsp_toggle` | pass |
| Copilot `e2e_lsp_write_diagnostics_with_copilot_gpt5_mini` | pass (~18s) |

## Activation / Surfacing

| Method | Effect |
|--------|--------|
| Default | `lsp.enabled: true` in `config.yaml` |
| `/lsp status` | Shows enabled state, timeout, server count, post-write behaviour |
| `/lsp on` / `/lsp off` | Session + `~/.edgecrab/config.yaml` |
| `/lsp toggle` | Flip enabled bit |
| `EDGECRAB_LSP_ENABLED=0\|1` | Env override on load |
| `lsp.enabled: false` in YAML | Hermes-equivalent static disable |

## Brutally Honest Hermes Parity Assessment

| Capability | Hermes | EdgeCrab | Verdict |
|------------|--------|----------|---------|
| Post-write LSP on write/patch | yes | yes | **Parity** |
| Delta + line_shift | yes | yes | **Parity** |
| `lsp_diagnostics` block + truncation | yes | yes | **Parity** |
| `snapshot_baseline` | yes | yes | **Parity** |
| Config `lsp.enabled: false` | yes | yes | **Parity** |
| Runtime slash toggle | no dedicated command | **`/lsp`** CLI + gateway | **EdgeCrab ahead** |
| Syntax-tier gate before LSP | yes (lint) | no | **Minor gap** |
| Multi-file patch LSP rollup | combined | first file only | **Minor gap** |

**Overall:** Matches or exceeds Hermes on the core contract. Surfacing is explicit via `/lsp` and status output; remaining gaps are edge cases.

## Residual Gaps

1. Lint/syntax gate before invoking LSP (Hermes skips LSP when parse lint fails).
2. Combined `lsp_diagnostics` across all files in a single `apply_patch` result.
3. Live E2E with real `rust-analyzer` is environment-dependent (mock + Copilot E2E prove wiring).
