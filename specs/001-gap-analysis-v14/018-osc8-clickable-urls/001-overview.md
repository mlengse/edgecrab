# 018 — OSC-8 Clickable URLs in TUI

**Tier:** B | **Impact:** 3 | **Value-per-Effort:** 5 | **Risk:** 1
**Primitive moved:** Trust (UX polish, anchored references)

## Why It Matters (First Principles)

When the agent emits a URL or a `file://path:line` reference, the user
should *click it* and have the terminal hand it to the OS. Modern
terminals (iTerm2, Wezterm, Kitty, Ghostty, Alacritty 0.13+, Windows
Terminal) implement [OSC 8 hyperlinks](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda)
— a printf escape sequence wrapping any text as a clickable hyperlink.

Hermes v0.14 wraps URLs and `path:line` refs in OSC 8 automatically.
Result: chat output is no longer wall-of-text — every reference is one
keystroke from the user's editor or browser.

## The Gap

EdgeCrab renders URLs and file refs as plain text. The user must
copy-paste.

## What EdgeCrab Gets Wrong Today

A response like `"see crates/edgecrab-core/src/agent.rs:142"` should be
clickable → opens VS Code at line 142. Today it's plain text.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
