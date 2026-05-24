# 023 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-tools/src/tools/browser/ (refactor into module)   │
   │                                                                  │
   │   mod.rs                                                         │
   │   cdp_pool.rs        — CdpPool { ws: Option<WsStream>,           │
   │                                   targets: HashMap<TabId,...> } │
   │   client.rs          — high-level CdpClient { pool: Arc<Mutex>}  │
   │   tools/                                                         │
   │     navigate.rs                                                  │
   │     screenshot.rs                                                │
   │     click.rs                                                     │
   │     evaluate.rs                                                  │
   │     console.rs                                                   │
   │                                                                  │
   │   Each tool: let client = ctx.browser_pool.client().await?;     │
   │              client.navigate(url).await?;                        │
   └──────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌──────────────────────────────────────────────────────────────────┐
   │       edgecrab-tools/src/registry.rs (ToolContext extension)     │
   │                                                                  │
   │   ToolContext { browser_pool: Arc<CdpPool>, ... }                │
   │   pool created lazily on first browser tool call                 │
   │   shared via Arc; per-session                                    │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **Refactor `browser.rs`** | into `browser/` module |
| **CDP pool** | `crates/edgecrab-tools/src/tools/browser/cdp_pool.rs` — connection holder, `connect_or_reuse()`, keep-alive task |
| **Client wrapper** | `crates/edgecrab-tools/src/tools/browser/client.rs` — high-level CDP ops over the pool |
| **Per-tool files** | one file per browser action; each is thin, calls client methods |
| **`ToolContext::browser_pool`** | `Arc<CdpPool>`; one per `Agent` instance |
| **Profile dir** | `~/.edgecrab/browser/profile-<session>/`; cleaned on session end (configurable retention) |
| **Crash recovery** | on WS error, next call triggers `connect_or_reuse()` which reconnects; ongoing tool returns transient error so loop retries |
| **Keep-alive** | spawn 30s interval ping task per pool; cancelled on Agent drop |
| **Cleanup** | implement `Drop` on `CdpPool` to send WS close + kill spawned Chrome subprocess |
| **Config** | `tools.browser.profile_retention: session|persistent`, `tools.browser.headless: true`, `tools.browser.timeout_ms: 30000` |

## Crate Choice

Use `chromiumoxide` or `headless_chrome` (re-evaluate maintenance
status; pick the one with active 2024+ releases and full async
support).

## Risks

- Chrome crashes mid-session; we must reap orphaned processes (PIDs
  tracked in `ProcessTable`).
- Memory growth from accumulated tabs; bound `targets` size, close
  oldest LRU when exceeded.
- Windows: child-process termination semantics differ; use
  `job_object` API to ensure cleanup on parent exit.

## DRY / SOLID Notes

- **SRP:** pool owns WS lifecycle; client owns RPC; tools own user
  intent.
- **DIP:** tools depend on `BrowserClient` trait → swap implementation
  for tests (mock).

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
