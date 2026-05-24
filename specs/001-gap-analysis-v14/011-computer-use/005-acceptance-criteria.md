# 011 — Acceptance Criteria

## Functional (macOS first)

- [ ] `computer_use({action:"screenshot"})` returns a `screenshot_path`.
- [ ] Next agent turn sees the screenshot in context.
- [ ] `computer_use({action:"click", coordinate:[x,y]})` produces a real
      click registered by a test app.
- [ ] `computer_use({action:"type", text:"hello"})` types into the
      focused field.
- [ ] `computer_use({action:"key", key:"cmd+s"})` triggers save in a
      test app.
- [ ] Permission denied → tool returns actionable error referencing
      System Settings → Privacy & Security → Screen Recording.

## Compression

- [ ] After 4 turns with screenshots, history retains only the last 3.
- [ ] Total context tokens stay within configured limit.

## Safety

- [ ] Tool disabled by default; explicit opt-in required.
- [ ] Destructive key combos blocked without `--yolo`.
- [ ] `/computer status` shows enabled state + permissions clearly.

## Multi-OS (Phased)

- [ ] macOS — phase 1 ship.
- [ ] X11 Linux — phase 2.
- [ ] Wayland (PipeWire portal) — phase 3.
- [ ] Windows — phase 4.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] Drivers gated behind `cfg(target_os = ...)` so non-target builds
      don't pull unused deps.
- [ ] Mock driver in tests; real driver behind `#[ignore]` for manual
      runs only.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
