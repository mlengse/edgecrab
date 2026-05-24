# 011 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| `computer_use` tool | `hermes-agent/tools/computer_use.py` |
| Backend abstraction | wraps `cua-driver` (Python) — provider-agnostic |
| Screenshot capture | platform-specific: `mss` (cross-platform), `screencapture` (macOS), `import` (Linux/X), `gnome-screenshot` |
| Action emission | Click, type, scroll, key combo — mapped to `pyautogui` / native APIs |
| Vision call | passes screenshot + recent action history to the *active* LLM (provider-agnostic) — same `ctx.llm` plumbing from feature 009 |

## Tool Surface (sketch)

```
computer_use({
  action: "screenshot" | "click" | "type" | "scroll" | "key" | "drag",
  coordinate?: [x, y],
  text?: string,
  scroll_amount?: number,
  scroll_direction?: "up"|"down"|"left"|"right",
  key?: string  // e.g. "cmd+s"
}) -> {
  screenshot_path?: string,
  status: "ok" | "error",
  message?: string
}
```

## Loop Pattern

The agent loop typically goes:
1. `computer_use({action:"screenshot"})`
2. multimodal LLM analyses image, decides action
3. `computer_use({action:"click", coordinate:[x,y]})`
4. another screenshot
5. ... until task complete

This means each turn carries one screenshot in the message history; the
compressor (feature 004 + existing) must strip old screenshots after N
turns to avoid token blow-up.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
- LLM-handle dependency: [../009-pluggable-providers-plugins/](../009-pluggable-providers-plugins/)
