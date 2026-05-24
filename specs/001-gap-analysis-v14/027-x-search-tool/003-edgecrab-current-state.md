# 027 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Web search tools | `crates/edgecrab-tools/src/tools/web.rs` |
| SSRF guard | `crates/edgecrab-security/src/ssrf.rs` |

## What Is Missing

1. `x_search` tool.
2. X API client.

## Honest Assessment

Simple REST tool. Cost is moderate — X API is expensive (`Pro` plan
required for production use). Document cost clearly.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
