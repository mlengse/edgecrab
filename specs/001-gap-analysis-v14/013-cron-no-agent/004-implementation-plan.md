# 013 — Implementation Plan

## Architecture (ASCII)

```
   ┌───────────────────────────────────────────────────────────────────┐
   │                       edgecrab-cron                               │
   │                                                                   │
   │   schedule.rs       (cron expr parsing — existing)                │
   │   store.rs          (SQLite-backed store — existing)              │
   │   scheduler.rs      (tick loop — EXTRACT from TUI)                │
   │   dispatcher.rs     (spawn agent, persist outcome) — NEW          │
   │   lock.rs           (file lock at ~/.edgecrab/daemon.lock) — NEW  │
   └───────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌───────────────────────────────────────────────────────────────────┐
   │                       edgecrab-cli                                │
   │                                                                   │
   │   main.rs                                                         │
   │     edgecrab            (TUI — existing)                          │
   │     edgecrab daemon     (NEW — runs scheduler in foreground)      │
   │     edgecrab daemon install   (NEW — writes launchd/systemd unit) │
   │     edgecrab daemon status    (NEW — shows lock state + pid)      │
   │     edgecrab daemon stop      (NEW — graceful shutdown)           │
   │                                                                   │
   │   TUI cron view: read-only over the same store                    │
   └───────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **New module** | `crates/edgecrab-cron/src/dispatcher.rs` — `async fn dispatch(job, store, app_config) -> JobOutcome` |
| **Extract** | `crates/edgecrab-cron/src/scheduler.rs` — tick loop currently in TUI |
| **New module** | `crates/edgecrab-cron/src/lock.rs` — `fs2`-style flock; refuses to start if held |
| **New migration** | `crates/edgecrab-state/migrations/NNN_cron_runs.sql` — `job_runs` table (job_id, started_at, finished_at, cost_usd, status, output_excerpt) |
| **New CLI** | `crates/edgecrab-cli/src/daemon.rs` — `edgecrab daemon` + install/status/stop |
| **Templates** | `crates/edgecrab-cli/templates/launchd.plist.tmpl` + `systemd.service.tmpl` |
| **TUI** | `crates/edgecrab-cli/src/views/cron.rs` — list jobs + recent runs (read-only when daemon owns the lock) |
| **Platform notification** | `dispatcher.rs` resolves `job.notify_on` list and calls `GatewaySender` if available |
| **Tests** | end-to-end: schedule a job, advance fake clock, assert outcome persisted + notification dispatched |

## Lock Semantics

- Daemon acquires exclusive lock at startup; release on graceful exit
  (signal handler) or process death (kernel releases flock).
- TUI never acquires the lock — it just reads the store.
- `/cron list` works from either context.

## Job Outcome Persistence

```sql
CREATE TABLE job_runs (
  id            INTEGER PRIMARY KEY,
  job_id        INTEGER NOT NULL REFERENCES cron_jobs(id),
  started_at    INTEGER NOT NULL,  -- epoch seconds
  finished_at   INTEGER,
  status        TEXT NOT NULL,     -- 'running'|'ok'|'error'|'timeout'
  cost_usd      REAL,
  input_tokens  INTEGER,
  output_tokens INTEGER,
  output_excerpt TEXT,             -- first 2 KB of final assistant message
  error_message TEXT
);
CREATE INDEX idx_job_runs_job ON job_runs(job_id, started_at DESC);
```

## OS Integration

- macOS: `~/Library/LaunchAgents/com.edgecrab.daemon.plist` template
  with `KeepAlive`, `ProgramArguments=[edgecrab, daemon]`, stdout/stderr
  to `~/.edgecrab/logs/daemon.log`.
- Linux: `~/.config/systemd/user/edgecrab-daemon.service` template with
  `Restart=on-failure`, `StandardOutput=append:%h/.edgecrab/logs/daemon.log`.
- Windows: `edgecrab daemon install` writes a Task Scheduler XML
  (phase 2, not blocker).

## DRY / SOLID Notes

- **SRP:** scheduler ticks; dispatcher runs; lock guards. Three files,
  three concerns.
- **DIP:** dispatcher depends on `AgentFactory` trait so tests can
  inject a mock factory.
- **DRY:** `JobOutcome` serialisation reuses `pricing::Cost` from core.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
