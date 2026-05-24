# 022 — Hermes Reference

| Concern | Hermes change |
|---------|---------------|
| Lazy tool registration | Tools register their *names* eagerly; their `schema()` and `execute()` lazily on first use |
| Deferred catalog merge | User overrides merged in background thread; first prompt uses embedded default |
| Skills lazy scan | Skills summary fetched async post-launch; first paint uses cached summary from `~/.hermes/skills/.cache.json` |
| Context-file cache | AGENTS.md content + injection-scan result cached by (path, mtime, size); skips re-scan on unchanged file |
| Startup profiler | `hermes --profile-startup` prints a flamegraph-like breakdown |

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
