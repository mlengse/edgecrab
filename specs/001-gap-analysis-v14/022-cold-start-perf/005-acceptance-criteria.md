# 022 — Acceptance Criteria

## Functional / Performance

- [ ] Cold-launch TTFP (no caches) reduced by ≥ 30% on a fixture with
      30 skills + 5 KB AGENTS.md.
- [ ] Warm-launch TTFP (caches valid) reduced by ≥ 50%.
- [ ] `edgecrab --profile-startup` prints phase breakdown.
- [ ] No correctness regression: first user message still uses fully
      merged catalog and final system prompt.

## Cache Invariants

- [ ] Modifying AGENTS.md externally invalidates the context-file
      cache entry on next launch (mtime-based).
- [ ] Adding a new skill triggers skills cache refresh next launch.
- [ ] Corrupt cache file → graceful fallback to full scan, log warn.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] All `OnceLock`s are infallible after first init.
- [ ] No unsafe.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
