# 006 — Acceptance Criteria

## Functional

- [ ] After 25 checkpoints in one session, exactly 20 remain (oldest
      5 evicted).
- [ ] After saving a checkpoint > 200 MB worth of new bytes in one
      session, older checkpoints evict until under cap.
- [ ] `target/`, `node_modules/`, `.venv/`, `.git/` are excluded by
      default. Verified via manifest diff.
- [ ] Hard-link Δ: a 100 MB workspace with 1 changed file produces a
      checkpoint of ≤ 1 MB additional disk (verified with `du`).
- [ ] `/rollback pin <n>` survives eviction.
- [ ] Restore is atomic (workspace never observable in half-restored state).
- [ ] Restore emits mutation records consumed by feature 002.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] No file > 300 lines in the checkpoint module (SRP enforcement).
- [ ] ≥ 10 tests covering eviction, pinning, hard-link Δ, excludes,
      atomic restore, two-session isolation, corrupt-index recovery.

## Operational

- [ ] On startup, EdgeCrab runs a `prune()` for the active session if the
      previous run crashed mid-write.
- [ ] `/rollback` shows total disk usage per checkpoint in the listing.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
