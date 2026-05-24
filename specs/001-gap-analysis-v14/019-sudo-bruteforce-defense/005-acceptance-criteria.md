# 019 — Acceptance Criteria

## Functional

- [ ] Default mode (`block`): any `sudo …` invocation returns
      `ToolError::Forbidden`; no `sudo` process spawned (verified by
      pid table observation in test).
- [ ] Mode `confirm`: TUI shows modal; on "yes" command runs; on
      "no"/timeout it doesn't.
- [ ] Mode `allow` + allowlist match: runs without prompt.
- [ ] Mode `allow` + no match: falls through to `confirm`.
- [ ] After `max_per_session: 1`, second sudo attempt → forced Block
      with "brute force limit" reason, regardless of mode.
- [ ] `sudo -S` with short non-newline stdin → Block("password injection").
- [ ] `--yolo` does NOT auto-allow sudo; still confirms.
- [ ] Non-interactive context (cron daemon) → Block, clear LLM-facing
      message.

## Security Logging

- [ ] Each sudo decision is logged with `command`, `mode`, `decision`,
      `reason`.
- [ ] No password content ever logged (verify with stdin containing
      "hunter2" → string never appears in logs).

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] ≥ 15 tests covering the decision matrix.

## Documentation

- [ ] `AGENTS.md` security section adds the sudo policy table.
- [ ] Default mode + how to opt into a more permissive mode documented.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
