# 016 — Acceptance Criteria

## Functional

- [ ] First message to a Discord channel triggers a 50-message backfill.
- [ ] Marker persisted; subsequent restarts do not re-backfill.
- [ ] `/backfill 200` fetches 200 messages and prepends to session.
- [ ] Bot's own messages mapped to `assistant`; humans to `user`.
- [ ] Attachments noted as text placeholders, not downloaded.
- [ ] Token budget enforced via prune-on-seed when overflow.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] ≥ 6 tests in `backfill.rs`; discord-specific integration test
      with mocked HTTP.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
