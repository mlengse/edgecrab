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

## Brutal Assessment vs Hermes (2026-05-24 refresh)

### What EdgeCrab does better

- **Operator UX**: `/computer setup|install|status|open`, TUI overlay, doctor — Hermes is CLI-string oriented.
- **Rust MCP client**: Simpler than Hermes `_AsyncBridge` + daemon thread; native async.
- **Screenshot disk cache** with `screenshot_path` in envelope — useful for debugging.

### Honest gaps (before this pass)

| Issue | Root cause | Severity |
|-------|------------|----------|
| **Sticky focus lost** | `capture()` always retargeted frontmost window → `key` hit ExpressVPN (pid 1391) after `focus_app` Safari (99826) | **Critical** |
| **Wrong z-order** | Sorted by `window_id` not `z_index` | High |
| **`list_apps` always `[]`** | Only parsed JSON arrays; cua-driver returns text lines | High |
| **268k token blow-up** | Spill preview included full base64 line; multimodal spilled → no images | Critical |
| **No `COMPUTER_USE_GUIDANCE`** | Agent not told `app=Safari` on capture | High |
| **Triple pruning** | Every-turn prune + spill + compression (Hermes also messy) | Medium |
| **Global backend singleton** | No lifecycle `stop()`, serializes parallel tools | Medium |
| **Element bounds always 0** | Parser incomplete | Medium |
| **`raise_window` no-op** | Schema lies | Low (intentional) |

### Fixes in this pass

1. **Sticky window context** — `capture()` preserves `active_pid`/`active_window_id` or `last_app` when `app=` omitted; `capture_after` uses `targeted_app()` (Hermes parity).
2. **z_index sort** — frontmost window selection matches Hermes.
3. **`list_apps` text parsing** — Hermes-compatible regex.
4. **Spill exempts `computer_use` multimodal** — images reach `tool_result_from_output`; preview uses `text_summary` only.
5. **`edgecrab_types::multimodal`** — DRY parse helpers shared by message, spill, tools.
6. **`COMPUTER_USE_GUIDANCE`** — injected when `computer_use` tool active.

### Still not Hermes-parity (honest backlog)

- Per-turn aggregate spill budget (`enforce_turn_budget`)
- Provider session cache for “reject multimodal tool content” 400 recovery
- `raise_window` implementation (if cua-driver supports it)
- Element bounds from AX tree
- Screenshot cache TTL / max files
- Session-scoped backend instead of `OnceLock` singleton
- Bundled `apple-macos-computer-use` skill
- Image-aware token estimator in compression (flat 1500/image)

### Verdict

**Core tool parity: yes. Production polish: now much closer.** The screenshot failure (ExpressVPN vs Safari) was a **first-principles state bug**, not “the model is dumb” — we were driving the wrong PID. Fix that before blaming permissions or vision routing.

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
