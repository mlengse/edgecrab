# 018 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Markdown renderer | `crates/edgecrab-cli/src/` (ratatui paragraph renderer) |
| Skin engine | `crates/edgecrab-cli/src/skin_engine.rs` |
| Terminal capability detect | partial — `IS_TERMUX` etc. |

## What Is Missing

1. No OSC 8 emitter.
2. No URL / `path:line` regex extraction in the render pipeline.
3. No terminal capability detection for OSC 8 specifically.
4. No editor-scheme preference (`config.terminal.editor_scheme`).

## Honest Assessment

Two regexes, one helper, one render hook. Cheapest polish in the
analysis — every keystroke saved per response is real user value.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
