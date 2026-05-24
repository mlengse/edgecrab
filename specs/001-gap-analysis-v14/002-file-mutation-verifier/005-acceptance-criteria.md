# 002 — Acceptance Criteria

## Functional

- [ ] After a turn that calls `file_write` once, the user sees a footer with
      `A path/to/new.rs +N` (or `M ... +N −M`).
- [ ] After a read-only turn (no mutations), **no** footer is rendered.
- [ ] Footer is also injected into history so turn N+1's model can reference it.
- [ ] Buffer is reset between turns (no leak across user messages).
- [ ] Works for `file_write`, `file_patch`, and any future mutation tool
      that opts in by calling `ctx.mutation_buffer.push(...)`.

## Performance

- [ ] Buffer is bounded (≤ 256 records/turn); excess records collapse into
      "+ N more" line.
- [ ] Adds < 5 ms to a turn that mutates 10 files.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings` clean.
- [ ] Unit tests in `mutations/mod.rs`: empty render, single add, mixed
      kinds, overflow collapse.
- [ ] No `unwrap()` in mutation code paths.

## UX

- [ ] Footer renders correctly on Termux compact UI (width < 60).
- [ ] Gateway platforms (Telegram, Slack) receive footer as plain text.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
