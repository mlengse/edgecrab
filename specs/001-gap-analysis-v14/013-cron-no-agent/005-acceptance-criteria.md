# 013 — Acceptance Criteria

## Functional

- [ ] `edgecrab daemon` starts and acquires the lock; second instance
      refuses with clear error.
- [ ] A job scheduled for `now + 10s` fires within the 11th second.
- [ ] Outcome persisted to `job_runs` with status, cost, excerpt.
- [ ] `notify_on: ["telegram:user_id"]` delivers a message to Telegram
      when the gateway is running.
- [ ] `edgecrab daemon install` writes the correct launchd/systemd unit
      for the platform.
- [ ] `edgecrab daemon status` shows pid, uptime, lock owner.
- [ ] `edgecrab daemon stop` triggers graceful shutdown; in-flight job
      completes before exit (or times out at configured limit).

## TUI / CLI

- [ ] `/cron list` displays scheduled jobs + last-run status.
- [ ] TUI cron view shows recent runs sorted desc.
- [ ] If daemon is down, `/cron status` says so and offers
      `edgecrab daemon install` hint.

## Reliability

- [ ] After kernel SIGKILL of daemon, lock is released (flock semantics).
- [ ] Daemon survives a single agent failure: one job erroring does not
      stop the tick loop.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] ≥ 15 tests; deterministic clock via injectable `Clock` trait.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
