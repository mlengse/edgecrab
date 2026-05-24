# 023 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Browser tool | `crates/edgecrab-tools/src/tools/browser.rs` |
| CDP client | per-call client creation (no pool) |
| `ToolContext` | already holds per-session state |

## What Is Missing

1. Persistent CDP WebSocket.
2. Per-session profile directory.
3. Target reuse / tab pooling.
4. WS crash recovery.
5. Keep-alive ping.

## Honest Assessment

Tractable rewrite of the browser tool module. The hard part is
ownership — pool lives in `ToolContext`, every tool fetches a handle.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
