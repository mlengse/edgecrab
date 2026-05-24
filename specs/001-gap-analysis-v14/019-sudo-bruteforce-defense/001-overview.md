# 019 — `sudo` Brute-Force Defence

**Tier:** B | **Impact:** 4 | **Value-per-Effort:** 5 | **Risk:** 1
**Primitive moved:** Security (hardening) + Trust

## Why It Matters (First Principles)

The `terminal` tool can — and does, when the LLM thinks it's helpful —
shell out commands like `sudo apt install ...`. If the user happens
to have a sudo session active, this is unauthorised privileged action.
If the user *doesn't* have an active session, sudo prompts for a
password. A pathological agent could attempt a series of guessed
passwords (a brute force) — and on default sudo configs this WILL get
the account locked out, or worse, the password attempted is logged
and visible.

Hermes v0.14 hardened this: any `sudo` invocation is intercepted by
the `terminal` tool before execution and either:
- blocked entirely (default),
- prompted to the user interactively for confirmation,
- allowed silently only if `--yolo` AND an allowlist pattern matches.

Additionally: passwords supplied through `sudo -S` stdin are scanned
and refused.

## The Gap

EdgeCrab's `terminal` tool runs `sudo` like any other command. No
interception, no confirmation, no brute-force defence.

## What EdgeCrab Gets Wrong Today

Worst case: the LLM, trying to fix a "permission denied" error, tries
`echo guess1 | sudo -S apt install ...` then `echo guess2 | sudo -S ...`.
This is a real (if low-probability) vector and the OS-level fallout
is severe (PAM lockout, log spam, possible alerting on enterprise
EDR systems).

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
