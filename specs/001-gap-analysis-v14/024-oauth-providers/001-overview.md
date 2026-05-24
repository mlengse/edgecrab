# 024 — OAuth Providers (Claude Pro / ChatGPT Pro / SuperGrok / Copilot)

**Tier:** C | **Impact:** 5 | **Value-per-Effort:** 4 | **Risk:** 3
**Primitive moved:** Access (consumer-tier inference)

## Why It Matters (First Principles)

Most users have a Claude Pro / ChatGPT Plus / SuperGrok subscription
but no API key. Providers expose chat endpoints behind OAuth that
unlock the *consumer flat-rate* — no per-token billing. Hermes v0.14
shipped OAuth flows for all four, enabling free-at-the-margin agent
runs for anyone with an existing subscription.

This is the *strategic* feature: it cuts running cost from $/M tokens
to "already paid".

## The Gap

EdgeCrab only supports API-key authentication. Users without a
billing-enabled API account cannot use top-tier models.

## What EdgeCrab Gets Wrong Today

Anthropic API key access requires a $5 deposit + monthly billing.
A Claude Pro subscriber pays $20/mo flat and has been *forced into a
second relationship* to use EdgeCrab. Same for ChatGPT, Grok, Copilot.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
- This is a **prerequisite** for: [../008-openai-compat-proxy/](../008-openai-compat-proxy/)
