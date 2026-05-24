# 013 — Cron Without an Active Agent (Headless Scheduler Daemon)

**Tier:** B | **Impact:** 4 | **Value-per-Effort:** 4 | **Risk:** 2
**Primitive moved:** Reliability (work happens even when nobody is watching)

## Why It Matters (First Principles)

A scheduled agent task is only useful if it runs **without a human at
the terminal**. Hermes v0.14 ships a headless cron daemon: a separate
process (`hermes daemon` or systemd/launchd unit) that owns the cron
table, fires jobs at their scheduled time, and dispatches each job into
a fresh agent invocation — with full results captured to the session
DB. The user can review what happened next time they open the TUI.

EdgeCrab has a `cron` tool and `edgecrab-cron` crate, but cron jobs
only execute while a TUI session is **open and idle on the cron tick**.
Close the terminal — nothing runs. This breaks the entire mental model.

## The Gap

EdgeCrab cron is tied to the TUI event loop. Hermes cron is a daemon.

## What EdgeCrab Gets Wrong Today

A user writes `cron_create({ when: "0 9 * * *", task: "morning brief" })`,
closes the terminal, and the next morning… nothing happened.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
