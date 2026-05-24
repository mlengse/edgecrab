# 011 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │              edgecrab-tools/src/tools/computer_use/              │
   │                                                                  │
   │   mod.rs          (ToolHandler entry; dispatches by `action`)    │
   │   driver.rs       (ComputerDriver trait — DIP boundary)          │
   │   drivers/                                                       │
   │     macos.rs      (cfg(target_os="macos") — CGDisplay + CGEvent) │
   │     x11.rs        (cfg(target_os="linux"); xcb crate)            │
   │     wayland.rs    (cfg(target_os="linux"); ashpd portal)         │
   │     windows.rs    (cfg(target_os="windows"); windows crate)      │
   │   permissions.rs  (TCC probe on macOS; portal probe on Wayland)  │
   │   compress.rs     (screenshot history pruning strategy)          │
   └──────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌──────────────────────────────────────────────────────────────────┐
   │      edgecrab-core/src/conversation.rs — multimodal handoff     │
   │                                                                  │
   │   - tool result with `screenshot_path` attached as image part   │
   │     in next assistant prompt                                    │
   │   - compress.rs hook removes screenshots older than N turns     │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **New module** | `crates/edgecrab-tools/src/tools/computer_use/mod.rs` |
| **Trait** | `crates/edgecrab-tools/src/tools/computer_use/driver.rs` — `trait ComputerDriver { fn screenshot() -> Image; fn click(p: Point); fn type_text(s: &str); fn scroll(...); fn key_combo(s: &str); }` |
| **macOS impl** | `drivers/macos.rs` — `core-graphics`, `core-foundation`, `cgevent` crates |
| **X11 impl** | `drivers/x11.rs` — `xcb` + `xtest` |
| **Wayland impl** | `drivers/wayland.rs` — `ashpd` (XDG desktop portal: Screenshot + RemoteDesktop) |
| **Windows impl** | `drivers/windows.rs` — `windows` crate, GDI + SendInput |
| **Permissions** | `permissions.rs` — `tcc-rs`-style probe on macOS; returns clear error if Screen Recording denied |
| **Compression** | `crates/edgecrab-core/src/compression.rs` — strip image parts older than `keep_last_n_screenshots` (default 3) |
| **Multimodal piping** | `crates/edgecrab-core/src/conversation.rs` — attach `screenshot_path` from tool result as image content block in next user/system message |
| **Slash command** | `/computer permissions` — runs probe and prints status |
| **Config** | `computer_use.enabled: false` by default; `computer_use.keep_last_n_screenshots: 3`; `computer_use.confirm_destructive: true` |
| **Safety** | a default `BlocklistDriver` wrapper that refuses dangerous keys (e.g. `cmd+shift+option+esc` macOS force-quit shortcut) unless `--yolo` |
| **Tests** | mock driver fixture; CI cannot test real screen capture (no display in CI) |

## Token Cost — Mandatory

A 1280×800 PNG screenshot at default quality is ≈ 1,500 image tokens
(Anthropic) or ≈ 1,000 tokens (OpenAI low detail). Three of them in
context = 4,500–5,000 tokens *per turn*. Compression must be aggressive:

- Default `keep_last_n_screenshots = 3`.
- Downsample to 1024×640 unless a tool argument requests full resolution.
- Strip screenshots that are not in the last N turns regardless of compressor state.

## Safety Defaults

- Tool disabled by default; opt-in via `config.yaml` or CLI flag.
- `/computer status` clearly shows: enabled/disabled, OS permissions
  granted/denied, driver in use.
- Destructive key combos require confirmation in interactive mode unless
  `--yolo`; in non-interactive mode they're blocked entirely.

## DRY / SOLID Notes

- **DIP:** `ComputerDriver` trait — tool depends on trait, not on
  per-OS module. Same pattern as `LLMProvider`.
- **OCP:** new OS = new file in `drivers/`. No changes to the tool.
- **SRP:** permissions, capture, action emission, compression — four
  modules, four concerns.
- **DRY:** the screenshot-attaching logic in `conversation.rs` reuses
  whatever pattern `vision` tool uses today (refactor both onto a
  shared helper).

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Uses LLM handle pattern from: [../009-pluggable-providers-plugins/](../009-pluggable-providers-plugins/)
