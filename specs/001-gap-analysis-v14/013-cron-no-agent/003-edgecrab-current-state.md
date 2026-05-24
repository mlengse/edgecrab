# 013 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Cron crate | `crates/edgecrab-cron/` |
| `cron` tool | `crates/edgecrab-tools/src/tools/cron.rs` |
| `/cron` slash command | `crates/edgecrab-cli/src/commands.rs` |
| Gateway lifecycle hooks | `crates/edgecrab-gateway/src/run.rs` |

## What Is Missing

1. No `edgecrab daemon` CLI subcommand.
2. No daemon process model — current scheduler lives inside the TUI
   process.
3. No launchd / systemd unit templates.
4. No lock file to prevent two daemons clobbering each other.
5. No `job_runs` table to capture outcomes.
6. No platform-emission integration (notify on Telegram on completion).

## Honest Assessment

`edgecrab-cron` was built with the right structure (cron expr parser
+ schedule store) but the **tick loop runs inside the TUI**. The fix
is small in lines, large in implications: extract the tick loop into
the daemon binary, have the TUI become a *read-only* consumer that
displays scheduled+past runs.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
