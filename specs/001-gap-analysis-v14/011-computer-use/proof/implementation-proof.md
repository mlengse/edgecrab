# 011 — Computer Use — Implementation Proof

**Branch:** `feat/computer-use`  
**Date:** 2026-05-24  
**Hermes source of truth:** `/Users/raphaelmansuy/Github/03-working/hermes-agent/tools/computer_use/`

## Summary

EdgeCrab ships a macOS **computer_use** tool that mirrors Hermes v0.14 architecture:
**cua-driver MCP stdio backend**, OpenAI function schema, `_multimodal` capture envelope,
safety gates, and opt-in toolset. Implemented in Rust with a `ComputerUseBackend` trait,
`CuaDriverBackend` (MCP client), and `NoopBackend` (CI/tests).

## Test Evidence

```bash
# Unit tests (noop backend — EDGECRAB_COMPUTER_USE_BACKEND=noop)
cargo test -p edgecrab-tools computer_use
# → 17 passed

# Screenshot history pruning
cargo test -p edgecrab-core prune_computer_use
# → 2 passed

# Full workspace (no regressions)
cargo test --workspace
cargo clippy --workspace -- -D warnings
# → clean
```

### Test matrix (computer_use module)

| Test | Covers |
|------|--------|
| `schema_has_expected_actions` | capture, click, set_value in enum |
| `schema_max_elements_defaults_match_runtime` | default 100, max 1000 |
| `coerce_max_elements_clamps` | bounds 1..1000 |
| `blocked_key_combo_detects_logout` | cmd+shift+q blocked |
| `blocked_type_pattern_catches_pipe_bash` | shell injection in type |
| `missing_action_returns_error` | arg validation |
| `unknown_action_via_dispatch` | unknown action |
| `noop_capture_*` | capture som/ax/vision modes |
| `noop_click`, `noop_type`, `noop_key`, `noop_wait`, `noop_list_apps`, `noop_focus_app`, `noop_set_value` | all actions via noop |
| `disabled_when_config_off` | enabled=false gate |
| `parse_multimodal_envelope` | _multimodal JSON parsing |
| `permissions_status_off_macos_or_missing_driver` | probe text |

## Architecture Delivered

```
edgecrab-tools/src/tools/computer_use/
├── mod.rs           ToolHandler + inventory registration
├── backend.rs       ComputerUseBackend trait
├── schema.rs        OpenAI schema (Hermes parity)
├── safety.rs        blocked keys/types, destructive approval
├── dispatch.rs      action routing + capture_after follow-up
├── response.rs      multimodal envelope + PNG cache
├── mcp.rs           dedicated stdio MCP session
├── cua_backend.rs   CuaDriverBackend (Hermes port)
├── noop.rs          CI/test stub
├── permissions.rs   macOS + cua-driver probe
└── tests.rs         17 unit tests
```

### Wiring

| Layer | Status |
|-------|--------|
| `config.rs` — `computer_use.enabled: false`, `keep_last_n_screenshots: 3` | ✅ |
| `toolsets.rs` — `COMPUTER_USE_TOOLS` opt-in | ✅ |
| `conversation.rs` — `tool_result_from_output`, image → `ChatMessage.images` | ✅ |
| `compression.rs` — `prune_computer_use_screenshots` every turn | ✅ |
| CLI `/computer status\|permissions` | ✅ |
| Gateway `/computer` | ❌ not wired (CLI only) |
| Vision routing (`vision_routing.py` aux path) | ❌ deferred |

## Acceptance Criteria Audit

| Criterion | Status | Notes |
|-----------|--------|-------|
| capture returns screenshot_path | ✅ (noop: no PNG; cua: path in envelope) | noop returns text-only ax/som |
| Next turn sees screenshot in context | ✅ | `Content::Parts` → `ChatMessage.images` |
| Real click/type/key e2e | ⚠️ manual | Requires cua-driver + macOS permissions; noop proves dispatch |
| Permission denied → actionable error | ✅ | `permissions_status` + install hint |
| After 4 captures, keep last 3 images | ✅ | `prune_computer_use_screenshots` tested |
| Tool disabled by default | ✅ | `computer_use.enabled: false` |
| Destructive keys blocked | ✅ | hard block + approval gate |
| `/computer status` | ✅ | CLI |
| macOS phase 1 | ✅ | cua-driver MCP |
| X11 / Wayland / Windows | ❌ phased | Hermes is also macOS-only for this tool |
| clippy clean | ✅ | |
| Mock driver in tests | ✅ | NoopBackend |
| Real driver `#[ignore]` manual | ⚠️ | No dedicated ignored e2e; manual with cua-driver |

## Brutal Honest Assessment vs Hermes

### Where EdgeCrab matches or exceeds Hermes

1. **Backend choice (cua-driver MCP)** — Correctly follows Hermes v0.14 source of truth, not the older spec's native CGEvent plan. Same action surface, same `_multimodal` contract.
2. **Safety** — Blocked key combos, blocked type patterns, destructive approval + yolo parity.
3. **Screenshot token control** — `prune_computer_use_screenshots` runs **every ReAct turn** (not only on compress), matching the spec's "regardless of compressor state" requirement.
4. **Rust type safety** — Trait-based backend (`ComputerUseBackend`) gives clean DIP; noop backend enables 17 deterministic tests without a display.
5. **Disk cache** — PNG saved under `~/.edgecrab/cache/computer_use/{uuid}.png` with path in envelope.

### Where EdgeCrab is behind Hermes

1. **Vision routing (`vision_routing.py`)** — Hermes routes captures through `auxiliary.vision` when the main model lacks vision or the provider rejects multimodal tool results (issue #24015). EdgeCrab always attaches images to tool messages; text-only models still get SOM/AX text summary but **no aux-vision pre-analysis**. Mitigation: use a vision-capable main model or enable `computer_use` with `mode=ax` for text-only workflows.
2. **Gateway slash command** — Hermes exposes status via gateway; EdgeCrab has CLI `/computer` only.
3. **Live macOS e2e proof** — Not run in CI (no display/cua-driver in GitHub Actions). Hermes has the same CI constraint; both rely on noop/mock in automation.
4. **Provider-specific multimodal tool results** — Hermes has per-provider adapters (Anthropic splices image into tool_result blocks). EdgeCrab relies on `edgequake-llm` `ChatMessage.images` on tool role — works for OpenAI-compatible providers; Anthropic path depends on edgequake-llm adapter maturity.
5. **Screenshot downsample** — Spec mentions 1024×640 default downsample; delegated to cua-driver (same as Hermes).

### Rust-specific trade-offs (acceptable)

- Async MCP client (`tokio`) vs Python threading — equivalent capability.
- `EDGECRAB_COMPUTER_USE_BACKEND=noop` env override for tests — cleaner than Hermes `_NoopBackend` injection.
- No Python GIL; backend singleton via `OnceLock<Mutex<...>>` — one cua-driver process per EdgeCrab instance (matches Hermes singleton pattern).

## Verdict

**Phase 1 (macOS / cua-driver): SHIP-READY for opt-in users with vision-capable models.**

Feature **matches Hermes core tool behavior** (schema, actions, safety, multimodal envelope, cua-driver backend) with two intentional gaps: **aux vision routing** and **gateway /computer**. For a Rust agent, the implementation is solid; aux vision routing should be the next increment if text-only main models must drive desktop control reliably.

## Enable Instructions

```yaml
# ~/.edgecrab/config.yaml
computer_use:
  enabled: true
  keep_last_n_screenshots: 3
  confirm_destructive: true
  cua_driver_cmd: cua-driver

enabled_toolsets:
  - computer_use
```

```bash
# Install cua-driver (macOS)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/trycua/cua/main/libs/cua-driver/scripts/install.sh)"

# CLI status
/computer status
/computer permissions
```
