# 030 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Hook signature | `def transform_output(text: str, ctx: PluginCtx) -> str` |
| Trigger | called once at end-of-turn (full message), once per streamed delta (token), or both (configurable per plugin) |
| Ordering | plugins applied in registration order; output of one feeds next |
| Examples shipped | inline-citation, profanity-filter, watermark, localiser |
| Performance | streamed-delta plugins must be O(token length); end-of-turn plugins get full text |

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
