# 018 — Acceptance Criteria

## Functional

- [ ] In iTerm2, a URL in agent output is clickable (cmd+click opens
      browser).
- [ ] A `crates/edgecrab-core/src/agent.rs:142` ref is clickable; with
      `editor_scheme: vscode`, opens VS Code at line 142.
- [ ] In a terminal without OSC 8 support, output is unchanged plain
      text (no garbage escape characters visible).
- [ ] Markdown link `[label](url)` is rendered as a clickable
      hyperlink labelled "label".
- [ ] Code blocks are NOT linkified (URL inside ``` stays as-is for
      copy fidelity).

## Capability Detection

- [ ] iTerm2 detected (`TERM_PROGRAM=iTerm.app`).
- [ ] WezTerm detected.
- [ ] Kitty detected.
- [ ] Windows Terminal detected (`WT_SESSION`).
- [ ] Plain `xterm-256color` without other hints → off.
- [ ] User override `cli.osc8_links: on` forces wrap.

## Width Correctness

- [ ] ratatui width / wrapping is unchanged when OSC 8 is enabled.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] Golden tests cover escape format byte-exactly.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
