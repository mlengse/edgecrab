# 021 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Curator agent | `hermes-agent/agents/curator.py` |
| Trigger | every Nth memory write OR every 24h whichever first |
| Model | cheap default (e.g. small Gemini / Haiku) |
| Output | rewritten MEMORY.md + an `archive/` directory of removed entries |
| Safety | dry-run + diff log to `~/.hermes/curator/runs/<ts>.json`; user can `hermes curator revert` |

## Curator Operations

1. Read current MEMORY.md.
2. Cluster bullets by topic.
3. Dedupe near-duplicates; merge into single entry with refs.
4. Mark entries older than 90d AND not referenced as "archive
   candidates".
5. Emit rewritten MEMORY.md.
6. Move archived entries to `archive/MEMORY-archive-<ts>.md`.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
