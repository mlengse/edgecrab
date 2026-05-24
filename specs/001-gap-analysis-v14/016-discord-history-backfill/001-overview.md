# 016 — Discord Channel History Backfill

**Tier:** B | **Impact:** 3 | **Value-per-Effort:** 4 | **Risk:** 1
**Primitive moved:** Trust (channel context understood from message 1)

## Why It Matters (First Principles)

A bot dropped into an existing Discord channel sees zero context. The
first conversation starts cold — the agent has no idea what's been
discussed for the last 3 weeks. Hermes v0.14 added **history backfill**:
on join (or `/backfill`), the adapter fetches the last N messages from
the channel via the Discord API and seeds them into the session as
context. The agent now has memory of what came before it.

## The Gap

EdgeCrab's Discord adapter starts every conversation cold.

## What EdgeCrab Gets Wrong Today

User in a long-running Discord channel says "remember when we agreed
on the postgres schema?" — the agent has nothing to reference and must
ask. Bad first impression. Easy fix.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
