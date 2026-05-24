# 009 — EdgeCrab Current State

| Existing | File |
|----------|------|
| Plugin crate (WASM + Lua) | `crates/edgecrab-plugins/` |
| Tool registry with `inventory::submit!` | `crates/edgecrab-tools/src/registry.rs` |
| `LLMProvider` trait (internal) | core providers |
| Model router | `crates/edgecrab-core/src/model_router.rs` |
| Agent builder | `crates/edgecrab-core/src/agent.rs` |
| Plugin slash command | `/plugins` (`crates/edgecrab-cli/src/plugins.rs`) |

## What Is Missing

1. **No public, stable `LLMProvider` trait** exported for external crates.
   The trait exists internally but isn't part of `edgecrab-sdk-core`.
2. **No runtime provider registration hook** — `model_router` resolves
   only baked-in providers.
3. **No tool override mechanism** — `ToolRegistry` keys on tool name
   uniquely; you cannot wrap a built-in tool without forking.
4. **No `ctx.llm` handle** — `ToolContext` doesn't expose a live LLM
   call surface, so a plugin can't do its own auxiliary reasoning.
5. **No entry-point-style auto-discovery for Rust plugins** — we have
   WASM + Lua, but native-Rust dynamic plugins (via `libloading`) aren't
   first-class.

## Honest Assessment

`edgecrab-plugins` exists but is **shallow**: tools-only and sandboxed.
The Hermes design has *escalating trust tiers*: WASM (safe) → Lua (mostly
safe) → native plugin (full trust, user-opted). EdgeCrab needs the native
tier with these three hooks to compete on extensibility.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
