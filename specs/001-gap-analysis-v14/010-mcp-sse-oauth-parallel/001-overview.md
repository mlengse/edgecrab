# 010 — MCP SSE Transport + OAuth Refresh + Parallel Tool Calls

**Tier:** A | **Impact:** 4 | **Value-per-Effort:** 4 | **Risk:** 2
**Primitive moved:** Reliability + Reach (MCP ecosystem)

## Why It Matters (First Principles)

The MCP ecosystem is rapidly standardising on three transports:

1. **stdio** — local subprocess (EdgeCrab has this ✅)
2. **HTTP JSON-RPC** — request/response (EdgeCrab has this ✅)
3. **HTTP+SSE** — server-pushed events; required for servers that emit
   progress, notifications, or long-running operations (**missing**)

In parallel, the MCP spec now mandates **OAuth 2.1 with PKCE refresh** as
the standard auth mechanism for hosted servers, and the spec encourages
**parallel `tools/call`** requests on a single connection. EdgeCrab's
current MCP client handles single-shot Bearer tokens and serial calls
only. Getting these three things right makes EdgeCrab compatible with
the entire Anthropic-hosted, Cloudflare-hosted, and Composio MCP catalog.

## The Gap

| Capability | EdgeCrab today | Required |
|------------|---------------|----------|
| SSE transport | ❌ | ✅ |
| Static Bearer | ✅ | ✅ |
| OAuth 2.1 + PKCE refresh | ❌ | ✅ |
| Parallel tools/call | ❌ (serial) | ✅ |
| `notifications/*` handling | ❌ | ✅ |

## What EdgeCrab Gets Wrong Today

`mcp_client.rs` opens an HTTP connection per tool call, sends a single
JSON-RPC request, awaits a single response, closes. SSE-only servers
return 400 immediately. OAuth-gated servers return 401 with no refresh
path. And when the agent emits 4 `tool_calls` in one turn, they execute
serially — wasting wall-clock time the user pays for.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
