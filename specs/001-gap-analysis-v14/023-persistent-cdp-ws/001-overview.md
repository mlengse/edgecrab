# 023 — Persistent CDP WebSocket

**Tier:** C | **Impact:** 3 | **Value-per-Effort:** 4 | **Risk:** 2
**Primitive moved:** Performance (capability latency)

## Why It Matters (First Principles)

Each `browser_*` tool call in EdgeCrab spawns a new Chrome DevTools
Protocol (CDP) session: handshake, target attach, page acquire. Even
locally that's 100–300 ms. Multi-step browser flows (navigate → wait
→ click → screenshot) pay that round trip 4 times.

Hermes v0.14 maintains a persistent CDP WebSocket per session, reusing
the same target across calls. Result: subsequent calls are 5–10 ms
instead of 200 ms. Hermes reported ~180× speedup for `browser_console`.

## The Gap

EdgeCrab's `browser` tools (in `crates/edgecrab-tools/src/tools/browser.rs`)
create a fresh client per call.

## What EdgeCrab Gets Wrong Today

A typical agent sequence "load page → screenshot → click → scrape" runs
~800 ms of CDP overhead alone. With pooling, ~210 ms.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
