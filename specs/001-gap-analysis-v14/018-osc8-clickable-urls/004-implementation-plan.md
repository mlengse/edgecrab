# 018 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │             edgecrab-cli/src/render/osc8.rs (NEW)                │
   │                                                                  │
   │   fn supports_osc8() -> bool                                     │
   │       match TERM_PROGRAM (iTerm.app, WezTerm, Ghostty, vscode)   │
   │       match TERM (xterm-kitty, alacritty if version >= 0.13)     │
   │                                                                  │
   │   fn wrap_url(url: &str, label: &str) -> String                  │
   │       "\x1b]8;;{url}\x1b\\{label}\x1b]8;;\x1b\\"                 │
   │                                                                  │
   │   fn linkify(text: &str, opts: LinkifyOpts) -> String            │
   │       1. find URLs   → wrap_url                                  │
   │       2. find file:line refs → resolve abs path                  │
   │          → editor-scheme URL → wrap_url                          │
   │       3. return rebuilt text                                     │
   └──────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌──────────────────────────────────────────────────────────────────┐
   │             edgecrab-cli/src/views/messages.rs                   │
   │                                                                  │
   │   when emitting a paragraph: if supports_osc8, linkify(text)     │
   │   before passing to ratatui Text/Paragraph widget.               │
   │                                                                  │
   │   ratatui's Span passes ANSI escapes through; the OSC 8          │
   │   sequence is invisible to its width calc → safe.                │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **New module** | `crates/edgecrab-cli/src/render/osc8.rs` |
| **Capability detect** | `supports_osc8()` reads `TERM_PROGRAM`, `TERM`, `WT_SESSION` (Windows Terminal), `KITTY_WINDOW_ID`, `ALACRITTY_LOG`; logs detected result once |
| **Workspace resolver** | helper to expand `relative/path:line` → absolute path using session CWD |
| **Editor scheme config** | `cli.editor_scheme: "vscode"` (default) / `"cursor"` / `"sublime"` / `"file"` |
| **Render hook** | call `linkify` in message renderer; gated by `cli.osc8_links: auto|on|off` |
| **Width safety** | ensure ratatui's width calculation isn't fooled by ANSI; if needed, post-process spans rather than raw strings |
| **Tests** | golden-file tests for the escape sequence; capability detect tests with mocked env vars |

## Edge Cases

- URLs already inside markdown links `[text](url)` — replace the
  *text* with an OSC 8 wrapper around the same text targeting `url`.
- Disable in ACP / gateway / SDK output — those consumers aren't
  terminals; only TUI render emits OSC 8.
- Markdown code blocks → do NOT linkify inside fenced code (preserves
  copy-paste fidelity).

## DRY / SOLID Notes

- **SRP:** `osc8.rs` only emits sequences; `linkify` only orchestrates;
  capability detection is its own function.
- **OCP:** new editor scheme = one entry in a static map.
- **DRY:** the URL regex is shared with the redaction patterns module
  (folder 017) — same source of truth.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
