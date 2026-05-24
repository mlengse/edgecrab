# 005 — `/handoff` Live Session Transfer

**Tier:** S | **Impact:** 4 | **Value-per-Effort:** 4 | **Risk:** 2
**Primitive moved:** Reliability + Cost

## Why It Matters

Sometimes Opus is overkill for the next 30 turns (you're done with the
architecture, you just need a Haiku to mass-rename files). Sometimes
your local profile hit a rate limit and you want to keep going on a
different OAuth provider. `/handoff` lets you swap the model/profile
**without losing conversation state, goals, todos, or cached prefix**.

## The Gap

EdgeCrab has `/model p/m` which **does** hot-swap the model. It does NOT:

1. Transfer state across **profiles** (only within one).
2. Cleanly summarise the in-flight task into a handoff brief.
3. Preserve prompt cache by carefully migrating the stable prompt block.
4. Surface in the gateway (Telegram, Slack) as a structured intent.

## What EdgeCrab Gets Wrong Today

`/model` swap mid-conversation works but the user experience is opaque:
no confirmation of what was preserved, no warning if the new model has a
smaller context window, no auto-compression if the history would exceed
the new window.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md) · [003-edgecrab-current-state.md](003-edgecrab-current-state.md) · [004-implementation-plan.md](004-implementation-plan.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Depends on: [004-prompt-prefix-cache/](../004-prompt-prefix-cache/)
- Composes with: [001-persistent-goals/](../001-persistent-goals/)
