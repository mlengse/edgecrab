# 020 — Session Auto-Resume

**Tier:** B | **Impact:** 3 | **Value-per-Effort:** 5 | **Risk:** 1
**Primitive moved:** Trust (continuity of attention)

## Why It Matters (First Principles)

A user closes the terminal after a long agent session, opens it again
five minutes later, and *wants the same conversation back*. The
default "fresh session every launch" model destroys context exactly
when continuity is most valuable. Hermes v0.14 added auto-resume:
if the previous session is "recent enough", offer to resume; else
start fresh.

## The Gap

EdgeCrab always starts a new session. The user must `/session list`,
find the right id, `/session switch <id>`. Friction kills the
behaviour.

## What EdgeCrab Gets Wrong Today

The session DB already has everything: timestamps, message history,
title, FTS index. We just don't wire it to launch UX.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
