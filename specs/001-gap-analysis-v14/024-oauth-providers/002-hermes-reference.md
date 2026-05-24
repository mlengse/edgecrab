# 024 — Hermes Reference

| Provider | Hermes module |
|----------|---------------|
| Claude Pro | `hermes-agent/providers/oauth/claude_pro.py` — PKCE OAuth via console.anthropic.com login flow, refresh token rotation |
| ChatGPT Pro | `hermes-agent/providers/oauth/chatgpt_pro.py` — login.openai.com PKCE; chat endpoint differs from API |
| SuperGrok | `hermes-agent/providers/oauth/super_grok.py` — x.ai OAuth |
| Copilot | `hermes-agent/providers/oauth/copilot.py` — github.com/login/device flow; token swap for Copilot chat token |

| Concern | Approach |
|---------|----------|
| Token storage | `~/.hermes/oauth/<provider>.json` chmod 0600 |
| Refresh | automatic on 401; rotate refresh token; fall back to re-auth on refresh failure |
| Endpoint differences | OAuth chat endpoints often differ from API URLs and have different headers; per-provider adapters |
| Rate limits | consumer plans have tighter limits; surface clearly in TUI |

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
