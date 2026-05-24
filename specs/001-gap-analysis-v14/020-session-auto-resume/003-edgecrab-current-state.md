# 020 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Session store | `crates/edgecrab-state/src/` (SQLite WAL + FTS5) |
| Setup wizard | `crates/edgecrab-cli/src/setup.rs` |
| `/session` slash | `crates/edgecrab-cli/src/commands.rs` |
| Per-session id | UUID stored in sessions table |

## What Is Missing

1. No `~/.edgecrab/last_session_id` pointer file.
2. No launch-time decision logic.
3. No prompt UI integrated into startup flow.
4. No config keys `session.auto_resume` / `session.auto_resume_max_age_secs`.

## Honest Assessment

A pointer file, three branches, one prompt. Cheapest continuity win.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
