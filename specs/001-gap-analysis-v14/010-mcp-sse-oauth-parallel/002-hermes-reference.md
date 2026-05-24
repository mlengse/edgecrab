# 010 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| MCP client base | `hermes-agent/tools/mcp_client.py` |
| SSE handler | Same module: `httpx` `AsyncClient` consuming `text/event-stream` |
| OAuth flow | `hermes-agent/hermes_cli/auth.py` — PKCE flow with browser launch + local callback server |
| Token refresh | Refresh-token store under `~/.hermes/mcp-oauth/<server>.json` |
| Parallel calls | `asyncio.gather(*[client.call_tool(t) for t in calls])` |

## Transport Selection

Hermes inspects the server URL's response on first contact:
- 200 with `Content-Type: text/event-stream` → SSE mode
- 200 with JSON-RPC response → plain HTTP mode
- 401 with `WWW-Authenticate: Bearer realm="..."` carrying OAuth metadata
  → trigger OAuth flow, retry

## OAuth Flow (PKCE)

```
client                 server                browser
  │ GET /mcp              │                     │
  │ ◄── 401 WWW-Authenticate (oauth metadata)   │
  │                       │                     │
  │ generate verifier/challenge                 │
  │ open browser ──────────────────────────────►│
  │                       │                     │ user authorises
  │ ◄────────── callback (code + state) ────────┤
  │ POST /token (code + verifier)               │
  │ ◄── access + refresh                        │
  │ store tokens chmod 0600                     │
  │ retry GET /mcp with Bearer                  │
  │ ◄── 200                                     │
```

On 401 during operation: try refresh; if refresh fails, re-trigger flow.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
