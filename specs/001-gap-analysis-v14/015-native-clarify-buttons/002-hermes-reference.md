# 015 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| `clarify` tool | `hermes-agent/tools/clarify.py` — emits a structured prompt with `options: List[ClarifyOption]` |
| Platform router | `hermes-agent/integrations/router.py` — inspects current platform context |
| Telegram render | `integrations/telegram/clarify.py` — `reply_markup` with `inline_keyboard` |
| Discord render | `integrations/discord/clarify.py` — `components` with `action_row` |
| Slack render | `integrations/slack/clarify.py` — block kit `actions` |
| Callback ingest | platform adapter receives the button tap as a synthetic user message with the option's `value` |

## ClarifyOption Shape

```
ClarifyOption {
  label: str,           # what the user sees on the button
  value: str,           # what gets sent back to the agent
  emoji?: str,
  style?: "primary" | "secondary" | "danger"
}
```

When the platform doesn't support buttons (SMS, plain webhook), the
adapter falls back to numbered text options.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
