# 011 — EdgeCrab Current State

| Existing | File |
|----------|------|
| `vision` tool | `crates/edgecrab-tools/src/tools/vision.rs` (image analysis only) |
| `browser` tool (CDP) | `crates/edgecrab-tools/src/tools/browser.rs` (headless Chrome only) |
| Image rendering in TUI | ratatui via kitty/iterm2 protocols |

## What Is Missing

1. No desktop screen capture primitive.
2. No native click/type/scroll primitives.
3. No provider-agnostic computer-use abstraction layer.
4. No multi-frame compression strategy for screenshot history.
5. No platform-detection: macOS uses `CGDisplay` APIs, Linux uses
   X11/Wayland (very different), Windows uses `GDI+`/`UIA`.

## Honest Assessment

Computer use is one of the higher-risk features in this analysis:
- macOS requires *Accessibility* and *Screen Recording* permissions
  (TCC) — the binary needs to be granted these by the user.
- Wayland on Linux blocks unprivileged screen capture by design.
- Permissions UX is fragile and easy to leave broken on first run.

Recommend: ship macOS first (clear permission story), then X11, then
Wayland (via PipeWire portal), then Windows. Document each clearly.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
