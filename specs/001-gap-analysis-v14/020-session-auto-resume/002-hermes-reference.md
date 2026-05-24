# 020 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Last-session pointer | `~/.hermes/last_session` (plain text id, atomic rename on update) |
| Decision | on launch: read pointer, look up `updated_at`, compare to `now - max_age_secs`; if fresh AND `auto_resume != never`, offer prompt |
| Modes | `prompt` (default), `always`, `never` |
| Max age | default 7200s (2 hours); configurable |
| UX | tiny "Resume previous session? (Y/n) — last active 3m ago, 14 messages" prompt at startup |

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
