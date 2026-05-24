# 002 — Acceptance Criteria

## Functional

- [x] After a turn that calls `file_write` once, the user sees a footer with
      `A path/to/new.rs +N` (or `M ... +N −M`).
- [x] After a read-only turn (no mutations), **no** footer is rendered.
- [x] Footer is also injected into history so turn N+1's model can reference it.
- [x] Buffer is reset between turns (no leak across user messages).
- [x] Works for `file_write`, `patch`, `apply_patch`, and any future mutation tool
      that opts in by calling `ctx.record_mutation(...)`.

## Performance

- [x] Buffer is bounded (≤ 256 records/turn); excess records collapse into
      "+ N more" line.
- [x] Adds < 5 ms to a turn that mutates 10 files.

## Code Quality

- [x] `cargo clippy --workspace -- -D warnings` clean.
- [x] Unit tests in `mutations.rs`: empty render, single add, mixed
      kinds, overflow collapse.
- [x] No `unwrap()` in mutation code paths.

## UX

- [x] Footer renders correctly on Termux compact UI (width < 60) via `render_success_footer_width`.
- [x] Gateway platforms (Telegram, Slack) receive footer as plain text.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
