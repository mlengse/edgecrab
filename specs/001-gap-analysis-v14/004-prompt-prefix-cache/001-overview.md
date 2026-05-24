# 004 — Cross-Session 1h Anthropic Prompt Prefix Cache

**Tier:** S | **Impact:** 5 | **Value-per-Effort:** 4 | **Risk:** 3
**Primitive moved:** Cost per useful turn

## Why It Matters

Anthropic's prompt cache cuts the cost of repeated input tokens by **90%**
and reduces latency by ~50%. Default cache lifetime is 5 minutes; the
**1-hour cache** (with `cache_control: {"type":"ephemeral","ttl":"1h"}`)
is the v0.14 cost-killer feature.

For a developer who opens 3 CLI sessions and 2 gateway conversations in a
workday — all sharing the same `SOUL.md`, `AGENTS.md`, skill summaries —
the system prompt prefix is **identical**. A 1h cache turns a 25k-token
prefix into 2.5k effective tokens across every session.

## The Gap

EdgeCrab already caches the system prompt **per session** (good — see
`AGENTS.md` "Prompt caching policy"). It does NOT:

1. Mark cache breakpoints with `cache_control: ttl: "1h"`.
2. Stabilise the prompt prefix across sessions (timestamps and per-session
   data leak into the cached region today).
3. Reuse the cache across CLI process restarts.

## What EdgeCrab Gets Wrong Today

`prompt_builder.rs` injects the *current* date/time into source 3 of the
system prompt. Every new session has a slightly different prefix → cache
miss → user pays full-input-token price on every cold start.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md) · [003-edgecrab-current-state.md](003-edgecrab-current-state.md) · [004-implementation-plan.md](004-implementation-plan.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Related: [005-session-handoff/](../005-session-handoff/) (handoff preserves cached prefix)
- Related: [001-persistent-goals/](../001-persistent-goals/) (goals must stay out of cached region)
