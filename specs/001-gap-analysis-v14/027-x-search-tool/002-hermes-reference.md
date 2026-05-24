# 027 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Tool | `hermes-agent/tools/x_search.py` |
| API | X v2 search/recent tweets endpoint |
| Auth | Bearer token (`X_BEARER_TOKEN`) |
| Args | `query`, `max_results` (10–100), `lang`, `since_id`, `time_range` |
| Output | list of {id, author, text, created_at, metrics, url} |
| Rate-limit handling | `Retry-After` aware, exponential back-off |

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
