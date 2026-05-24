# 011 — Computer Use — Implementation Proof

**Branch:** `feat/computer-use-ux`  
**Date:** 2026-05-24  
**Hermes source of truth:** `/Users/raphaelmansuy/Github/03-working/hermes-agent/tools/computer_use/`

## Summary

EdgeCrab ships macOS **computer_use** with Hermes v0.14 parity (cua-driver MCP, aux vision routing,
safety, screenshot pruning) plus a **polished operator UX**: structured readiness reports, TUI overlay
panel, `/computer enable|open|help`, gateway parity, and `edgecrab doctor` integration.

## Test Evidence

```bash
cargo test -p edgecrab-tools computer_use --lib   # 30 passed (+ 1 ignored manual e2e)
cargo test -p edgecrab-core prune_computer_use    # 2 passed
cargo test --workspace                            # all passed
cargo clippy --workspace -- -D warnings           # clean

# Manual live capture (macOS + cua-driver + permissions):
cargo test -p edgecrab-tools manual_e2e -- --ignored --nocapture
```

## UX Deliverables

| Surface | Behavior |
|---------|----------|
| TUI `/computer` | Opens **report overlay** (title + subtitle + body) like `/status` and `/cost` |
| `/computer status` | Readiness checklist: platform, driver, config, toolset, accessibility, vision routing |
| `/computer permissions` | Actionable permission checklist + install hint |
| `/computer open` | Opens Screen Recording + Accessibility System Settings panes |
| `/computer enable\|disable` | Persists `computer_use.enabled` + adds toolset; hot-updates live agent |
| `/computer help` | Full subcommand reference |
| Gateway `/computer` | Same text reports + enable/disable persist |
| `edgecrab doctor` | Checks computer_use readiness when enabled in config |

## Architecture (SOLID / DRY)

```
status.rs           — single source for reports (CLI, gateway, TUI overlay, doctor)
vision_routing.rs   — routing policy only
aux_vision.rs       — aux execution only
response.rs         — capture finalization only
permissions.rs      — low-level driver/platform probes
manual_e2e.rs         — #[ignore] live capture test
```

Shared helpers: `analyze_local_image` (vision.rs), `format_computer_command` / `computer_command_overlay`,
`AppConfig::persist_computer_use_enabled`, `Agent::set_computer_use_enabled`.

## Brutal Assessment vs Hermes

### Exceeds Hermes on UX

- **Structured readiness report** with `[ok]` / `[warn]` / `[fail]` markers and next-steps — Hermes prints ad-hoc strings.
- **TUI overlay panel** — Hermes TUI/gateway lack an equivalent polished status surface.
- **`/computer open`** — one command opens both Screen Recording and Accessibility panes.
- **`/computer enable`** — one command enables config + toolset + live session (Hermes requires manual config + toolset edit).
- **`edgecrab doctor`** integration when feature is enabled.

### Parity with Hermes core

| Area | Verdict |
|------|---------|
| cua-driver MCP backend | **Parity** |
| Action surface + schema | **Parity** |
| Safety + screenshot pruning | **Parity** |
| Aux vision routing (#24015) | **Parity** |
| `_multimodal` envelope | **Parity** |

### Remaining honest gaps

| Gap | Severity | Notes |
|-----|----------|-------|
| Screen Recording TCC preflight | Low | No public macOS API — same as Hermes; manual verify required |
| Live click/type e2e in CI | Medium | `manual_e2e` test exists but `#[ignore]`; run locally |
| Bundled macOS computer-use skill | Low | Hermes ships `apple-macos-computer-use` skill; EdgeCrab relies on tool schema |
| Linux/Windows phases | N/A | Hermes also macOS-only for this tool |

## Verdict

**Feature complete for Phase 1.** Matches Hermes tool behavior and **exceeds on operator UX** for setup,
diagnostics, and enablement. Production-ready for opt-in macOS users with cua-driver.

## Quick Start

```yaml
# ~/.edgecrab/config.yaml — or run /computer enable
computer_use:
  enabled: true
enabled_toolsets:
  - computer_use
```

```bash
/computer status       # TUI overlay readiness report
/computer open         # jump to privacy panes
/computer enable       # persist + activate
edgecrab doctor        # validates when enabled
```
