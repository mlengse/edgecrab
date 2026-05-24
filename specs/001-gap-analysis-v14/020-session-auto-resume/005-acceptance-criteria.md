# 020 — Acceptance Criteria

## Functional

- [ ] Fresh install: launch → new session (no pointer yet).
- [ ] After one chat exchange + quit + relaunch within 2h: prompt
      "Resume previous session? (Y/n)" with last-active duration.
- [ ] `Y` → resumes; chat history rendered; FTS works.
- [ ] `N` → new session.
- [ ] Mode `always` → resume without prompt.
- [ ] Mode `never` → always new session.
- [ ] After 2h+ inactivity → no prompt, new session.
- [ ] Session with > 200 messages → no auto-resume; user must `/session switch`.
- [ ] `edgecrab resume` subcommand forces resume.
- [ ] `edgecrab new` subcommand forces new.
- [ ] Pointer to deleted session → silent fall-through to new.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] Pointer write uses atomic rename (no partial writes on crash).
- [ ] Tests cover decision matrix.

## Documentation

- [ ] `AGENTS.md` config table adds `session.auto_resume` keys.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
