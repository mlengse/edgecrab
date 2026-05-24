# 010 — EdgeCrab Current State

| Existing | File |
|----------|------|
| MCP client | `crates/edgecrab-tools/src/tools/mcp_client.rs` |
| Static Bearer | `read_mcp_token` / `write_mcp_token` / `remove_mcp_token` |
| `/mcp-token` slash command | `crates/edgecrab-cli/src/commands.rs` |
| `/reload-mcp` | Drops all connections |
| Conversation tool dispatch (serial) | `crates/edgecrab-core/src/conversation.rs` |

## What Is Missing

1. **SSE transport** — `reqwest`-based event-stream consumer with
   reconnect/backoff.
2. **OAuth 2.1 + PKCE flow** — verifier/challenge, local callback HTTP
   listener (ephemeral port), browser launch, refresh-token storage.
3. **WWW-Authenticate parsing** — extract `authorization_uri`,
   `token_uri`, `scope` from the challenge header per RFC 9728.
4. **Parallel tool dispatch** — `execute_loop` currently runs each
   `tool_call` sequentially; should `join_all` independent calls.
5. **`notifications/*` plumbing** — surface progress to the TUI as
   `StreamEvent::ToolProgress`.

## Honest Assessment

The MCP client module is one of the cleaner ones in the codebase. The
additions are well-scoped: each is a separate sub-module. Parallel
dispatch is a small but tricky change — must respect tool-level
serialisation hints (some tools, like `terminal`, should not run in
parallel with themselves).

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
