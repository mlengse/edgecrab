# 005 — EdgeCrab Current State

| Existing | File |
|----------|------|
| `/model` hot-swap | `crates/edgecrab-cli/src/commands.rs` |
| Agent hot-swap | `crates/edgecrab-core/src/agent.rs` (`Agent::set_model`) |
| Provider factory | `crates/edgecrab-core/src/model_router.rs` |
| Model catalog (ctx windows) | `crates/edgecrab-core/src/model_catalog.rs` |

## What Is Missing

1. No handoff-brief auxiliary call.
2. No context-window check / auto-compress before swap.
3. No `/handoff` slash command (separate from `/model` which is implicit).
4. No gateway surface.
5. No persistence of "this session was handed off from X → Y" for `/insights`.

## Honest Assessment

`/model` works but is silent. Users don't trust silent swaps. The fix
is a small wrapper around the existing hot-swap that adds:
brief + window-check + confirmation.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
