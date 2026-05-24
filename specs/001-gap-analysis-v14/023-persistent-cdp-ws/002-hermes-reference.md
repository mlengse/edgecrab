# 023 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| CDP pool | `hermes-agent/tools/browser/cdp_pool.py` |
| Lifecycle | one CDP WS per session; keep-alive ping every 30s; reaped on session end |
| Target reuse | per-tab attachment cached by URL hash; navigate-in-place rather than new tab when possible |
| Crash recovery | on WS close, reconnect lazily on next call |
| Headless flags | shared by all calls — sandbox, profile dir under `~/.hermes/browser/profile-<session>` |

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
