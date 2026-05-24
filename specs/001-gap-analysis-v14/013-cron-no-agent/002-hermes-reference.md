# 013 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Daemon entry | `hermes daemon` subcommand starts an asyncio scheduler |
| Schedule store | SQLite table; daemon reads on startup + on inotify of the cron file |
| Tick loop | second-resolution loop: `await asyncio.sleep(1.0 - (now % 1))` |
| Job dispatch | for each due job, spawn a fresh `Agent` instance, run the configured prompt, persist result |
| OS integration | provides templates for `launchd` (macOS) plist and `systemd` user unit |
| Lock | `~/.hermes/daemon.lock` (flock) — only one daemon active per home dir |
| Logs | `~/.hermes/logs/daemon.log` (rotating, 10 MB × 5) |

## Dispatch Model

```
T=09:00:00
   daemon loop tick
   ├─► query store WHERE next_run <= now
   ├─► for each due job:
   │     update next_run = cron.next(now)
   │     spawn task → run agent with the configured prompt + system
   │     on completion: write summary + cost to job_runs table
   │     emit notification to all platforms in `notify_on` (Telegram, etc.)
   └─► sleep until next second boundary
```

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
