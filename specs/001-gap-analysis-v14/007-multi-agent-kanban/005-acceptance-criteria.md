# 007 — Acceptance Criteria

## Phase 1 (Cards)

- [ ] `kanban_create` adds a card with title/description/tags.
- [ ] `kanban_list { state: "TODO" }` returns the new card.
- [ ] `kanban_complete` transitions DOING → DONE.
- [ ] Cards persist across `edgecrab` restarts.

## Phase 2 (Leases)

- [ ] `kanban_claim` returns error if card already claimed and lease not expired.
- [ ] After `lease_secs + grace`, reaper releases the card.
- [ ] `kanban_heartbeat` extends the lease.

## Phase 3 (Dependencies)

- [ ] `kanban_create { depends_on: [parent_id] }` is rejected by `claim`
      until the parent is DONE.
- [ ] `kanban_list { ready: true }` excludes blocked cards.

## Phase 4 (Worker Auto-Spawn)

- [ ] `delegate_task` integration: a supervisor agent can `kanban_create`
      multiple cards and spawn N worker subagents that loop:
      claim → execute → complete.
- [ ] Concurrency cap enforced (`kanban.max_workers`).

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] ≥ 15 tests across phases.
- [ ] All five kanban modules ≤ 250 lines each.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
