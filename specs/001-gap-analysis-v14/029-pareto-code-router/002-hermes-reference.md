# 029 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Classifier | `hermes-agent/router/pareto.py` |
| Tiers | `quick` (greetings, lookups, sub-50-token answers) → cheap model; `default` (most chat) → mid model; `code` (file edits, multi-step plans) → strong model; `vision` (image present) → vision-capable model |
| Signals | regex on user input + presence of attachments + length + estimated tokens + tool history |
| Override | per-turn `@<model>` prefix forces routing |
| Reporting | `/router stats` shows turn counts and savings per tier |

## Classification Signals

- Length < 80 chars + no code fence + no question mark on technical
  keyword → likely `quick`.
- Code fence ≥ 200 chars OR file path attached → `code`.
- Image in message → `vision`.
- Otherwise → `default`.
- Tool history saturated (≥ 8 prior tool calls) → escalate to `code`
  (reasoning depth needed).

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
