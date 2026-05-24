# 011 — Computer Use — Implementation Proof

**Branch:** `feat/computer-use-phase2` (merged from `feat/computer-use`)  
**Date:** 2026-05-24  
**Hermes source of truth:** `/Users/raphaelmansuy/Github/03-working/hermes-agent/tools/computer_use/`

## Summary

EdgeCrab ships a macOS **computer_use** tool mirroring Hermes v0.14:
**cua-driver MCP stdio backend**, OpenAI function schema, `_multimodal` capture envelope,
**auxiliary vision routing** (#24015), safety gates, screenshot pruning, and opt-in toolset.

## Test Evidence

```bash
cargo test -p edgecrab-tools computer_use   # 25 passed
cargo test -p edgecrab-core prune_computer_use  # 2 passed
cargo test --workspace                      # all passed
cargo clippy --workspace -- -D warnings     # clean
```

## Architecture

```
edgecrab-tools/src/tools/computer_use/
├── mod.rs              ToolHandler + registration
├── backend.rs          ComputerUseBackend trait
├── schema.rs           OpenAI schema (Hermes parity)
├── safety.rs           blocked keys/types, destructive approval
├── dispatch.rs         action routing + capture_after
├── response.rs         finalize_capture_response (multimodal vs aux)
├── vision_routing.rs   should_route_capture_to_aux_vision (Hermes port)
├── aux_vision.rs       route_capture_through_aux_vision
├── status.rs           DRY /computer formatter (CLI + gateway)
├── mcp.rs              stdio MCP client
├── cua_backend.rs      CuaDriverBackend
├── noop.rs             CI/test stub
├── permissions.rs      macOS + cua-driver probe
└── tests.rs            25 unit tests
```

## Wiring Checklist

| Layer | Status |
|-------|--------|
| `computer_use.enabled: false` default | ✅ |
| `COMPUTER_USE_TOOLS` opt-in toolset | ✅ |
| Multimodal → `Message::tool_result_from_output` → `ChatMessage.images` | ✅ |
| `prune_computer_use_screenshots` every ReAct turn | ✅ |
| Aux vision routing for non-vision main models | ✅ |
| CLI `/computer status\|permissions` | ✅ |
| Gateway `/computer` | ✅ |
| `active_model` on `AppConfigRef` for routing | ✅ |
| Shared `analyze_local_image` in `vision.rs` (DRY) | ✅ |

## Brutal Assessment vs Hermes

### Matches or exceeds

| Area | Verdict |
|------|---------|
| cua-driver MCP backend | **Parity** — same architecture as Hermes v0.14 |
| Action surface + schema | **Parity** — capture/click/type/key/drag/scroll/set_value/list_apps/focus_app/wait |
| `_multimodal` envelope contract | **Parity** |
| Safety (blocked keys, type patterns, approval) | **Parity** |
| Screenshot history pruning (keep last N) | **Parity** — runs every turn, not only on compress |
| Vision routing policy (`vision_routing.py`) | **Parity** — explicit aux override, provider tool-result support, model capability |
| Aux vision pre-analysis (`_route_capture_through_aux_vision`) | **Parity** — reuses `vision_analyze` backend resolution |
| `/computer` status | **Parity** — CLI + gateway via shared `format_computer_command` |
| Rust type safety + noop CI backend | **EdgeCrab advantage** — 25 deterministic tests without display |

### Remaining gaps (honest)

| Gap | Severity | Mitigation |
|-----|----------|------------|
| Live macOS e2e (real click/type with cua-driver) | Medium | Manual only; same CI constraint as Hermes |
| Provider-specific Anthropic tool_result image splice | Low | Delegated to `edgequake-llm`; multimodal path works for OpenAI-compatible providers |
| Linux X11 / Wayland / Windows | N/A phase 1 | Hermes is also macOS-only for this tool |
| Screenshot downsample to 1024×640 | Low | Delegated to cua-driver (Hermes same) |
| `#[ignore]` manual e2e test harness | Low | Document manual procedure in enable instructions |

### First-principles / SOLID notes

- **DIP:** `ComputerUseBackend` trait; tool depends on abstraction, not OS APIs.
- **SRP:** routing policy (`vision_routing`), aux execution (`aux_vision`), response shaping (`response`), status formatting (`status`) are separate modules.
- **DRY:** `analyze_local_image` shared with `vision_analyze`; `format_computer_command` shared CLI/gateway; routing reuses `vision_models` capability policy.
- **Fail-closed routing:** When provider/model cannot accept tool images → aux vision; when aux fails → fall through to multimodal (Hermes same).

## Verdict

**Phase 1 complete.** Feature **matches or exceeds** Hermes computer_use for macOS/cua-driver workflows, including the #24015 aux-vision fix. Suitable for opt-in production use with text-only or vision main models.

## Enable

```yaml
computer_use:
  enabled: true
  keep_last_n_screenshots: 3

enabled_toolsets:
  - computer_use

auxiliary:
  provider: openai   # optional — forces aux vision for captures
  model: gpt-4o
```

```bash
/computer status      # CLI or gateway
/computer permissions
```
