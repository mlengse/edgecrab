# 010 — Acceptance Criteria

## Functional

- [ ] Configure an SSE-only MCP server in `~/.edgecrab/config.yaml`; tool
      calls succeed.
- [ ] SSE reconnect after server-side disconnect (10s within 3 attempts).
- [ ] OAuth flow: first call to an OAuth-gated server opens the browser,
      completes PKCE flow, persists tokens.
- [ ] Refresh: stored tokens with `expires_at` in the past are refreshed
      transparently on next call.
- [ ] Refresh failure → re-trigger interactive flow with clear message.
- [ ] 4 independent `tool_calls` in one turn dispatch in parallel
      (asserted via timing: each tool sleeps 200ms; total < 500ms).
- [ ] Serial-peer tools (`terminal` with `terminal`) batch correctly:
      two `terminal` calls in one turn execute serially even when other
      tools run in parallel.
- [ ] MCP `notifications/progress` arrives at TUI as
      `StreamEvent::ToolProgress` and renders.

## Security

- [ ] OAuth token files chmod 0600.
- [ ] OAuth `state` parameter validated on callback.
- [ ] OAuth callback listener binds 127.0.0.1 only.
- [ ] Tokens redacted from all log output.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] ≥ 25 tests in the `mcp/` module tree.
- [ ] No dependency added to `edgecrab-core` (all new deps stay in
      `edgecrab-tools`).

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
