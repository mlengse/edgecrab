# 010 — Implementation Plan

## Architecture (ASCII)

```
   ┌───────────────────────────────────────────────────────────────────┐
   │             edgecrab-tools/src/tools/mcp/                         │
   │                                                                   │
   │   mod.rs          (public: ToolHandler entry point)               │
   │   transport/                                                      │
   │     stdio.rs        ← existing                                    │
   │     http.rs         ← existing (request/response)                 │
   │     sse.rs          ← NEW (event-stream consumer)                 │
   │   auth/                                                           │
   │     bearer.rs       ← existing (token file)                       │
   │     oauth.rs        ← NEW (PKCE flow)                             │
   │     callback.rs     ← NEW (ephemeral local listener)              │
   │   client.rs         (connection pool, request multiplexer)        │
   │   notifications.rs  (handle notifications/* → StreamEvent::Progress)│
   └───────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌───────────────────────────────────────────────────────────────────┐
   │             edgecrab-core/src/conversation.rs                     │
   │                                                                   │
   │   for batch in group_independent(tool_calls):                     │
   │       results = join_all([dispatch(c) for c in batch]).await      │
   │       messages.extend(results.into_messages())                    │
   └───────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **Refactor** | Split `crates/edgecrab-tools/src/tools/mcp_client.rs` into module `mcp/` (mod.rs, transport/, auth/, client.rs, notifications.rs) — keep backward-compat re-exports |
| **New** | `mcp/transport/sse.rs` — `reqwest::Client::get(...).send().await?.bytes_stream()` parser; reconnects with exponential backoff; emits `JsonRpcMessage` enum |
| **New** | `mcp/auth/oauth.rs` — `oauth2` crate (or hand-rolled PKCE — small enough); challenge=`SHA256(verifier).base64url`; persist tokens at `~/.edgecrab/mcp-oauth/<server>.json` chmod 0600 |
| **New** | `mcp/auth/callback.rs` — bind 127.0.0.1:0 (ephemeral), serve `/callback`, await one request, return code+state |
| **New** | `mcp/auth/discovery.rs` — parse `WWW-Authenticate: Bearer ...` per RFC 9728; fetch `/.well-known/oauth-authorization-server` |
| **Modify** | `crates/edgecrab-core/src/conversation.rs` — `dispatch_tool_calls_parallel` helper; respects `ToolHandler::serial_with(&self) -> &[&'static str]` for tools that must not run concurrently with named peers |
| **Modify** | `crates/edgecrab-tools/src/registry.rs` — `ToolHandler::serial_with` default-empty method (OCP) |
| **Slash commands** | `/mcp auth <server>` (trigger OAuth interactively), `/mcp status` (list connections + auth state), `/mcp refresh <server>` |
| **Tests** | mock HTTP+SSE server; OAuth happy/refresh/expired paths; parallel dispatch determinism for independent calls |

## Parallel Dispatch — Safety Rules

Tools that must not run in parallel with themselves or peers:

| Tool | Serial peers |
|------|--------------|
| `terminal` | `["terminal", "execute_code"]` |
| `file_write` | `["file_write", "file_patch"]` (same path level — see [002-file-mutation-verifier/](../002-file-mutation-verifier/)) |
| `file_patch` | same |
| `browser_*` | `["browser_*"]` (single page state) |

Implementation: `group_independent(calls)` groups consecutive calls into
batches such that no two calls in the same batch claim a serial peer.

## OAuth Storage

```
~/.edgecrab/mcp-oauth/
├── server-a.json   (mode 0600)
│   { "access_token": "...", "refresh_token": "...",
│     "expires_at": "2026-05-25T12:00:00Z",
│     "token_uri": "https://...", "scope": "tools:call" }
└── server-b.json
```

On every request: if `expires_at - now < 60s`, refresh; on 401 mid-flight,
refresh once and retry.

## DRY / SOLID Notes

- **SRP:** transport / auth / client / notifications each in own module.
- **OCP:** `serial_with` default-empty addition to `ToolHandler` —
  existing tools unaffected.
- **DIP:** `conversation.rs` depends on `ToolRegistry::dispatch_many`
  (new), not on `mcp` internals.
- **DRY:** OAuth callback listener reusable for future OAuth providers
  (folder 024 — Claude Pro/ChatGPT Pro/Grok). Lift into
  `edgecrab-security/src/oauth_callback.rs` once two callers exist.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Callback infrastructure shared with: [../024-oauth-providers/](../024-oauth-providers/)
