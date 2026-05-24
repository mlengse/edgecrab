# 004 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Prompt cache markers | `hermes-agent/agent/prompt_caching.py` |
| Anthropic adapter | `hermes-agent/agent/anthropic_adapter.py` (attaches `cache_control` blocks) |
| Per-turn cache stats | `hermes-agent/agent/usage_pricing.py` |
| Prefix stability | `hermes-agent/agent/prompt_builder.py` keeps the cached prefix **time-free**; per-turn date/time is appended below the cache breakpoint as a fresh user message |

## Mechanism (verbatim concept)

Anthropic content blocks support `cache_control`:

```json
[
  { "type":"text", "text":"<stable identity + skills index>",
    "cache_control": { "type":"ephemeral", "ttl":"1h" } },
  { "type":"text", "text":"<volatile per-turn context>" }
]
```

The cached block is byte-identical across sessions → cache hit across
process restarts (within 1 h) → 90% input-token discount.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
