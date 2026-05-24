# 029 — Acceptance Criteria

## Functional

- [ ] With `router.auto: true`, "hi" routes to `Quick` tier model.
- [ ] A 300-line code fence routes to `Code` tier.
- [ ] An attached image routes to `Vision` tier.
- [ ] Session escalates to `Code` after 8 prior tool calls and stays
      (sticky).
- [ ] `@opus what is 2+2` overrides router; uses opus despite quick
      pattern.
- [ ] With `router.auto: false`, default model always used.
- [ ] `/router stats` shows turns per tier + estimated savings vs.
      always-default.
- [ ] Mid-stream model switch never happens.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] Classifier matrix tested with ≥ 20 fixtures.
- [ ] Override parser tested with edge cases (no-space `@`, multiple
      `@`, lone `@`).

## Documentation

- [ ] `AGENTS.md` adds router config block.
- [ ] `/router show` documented in `/help`.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
