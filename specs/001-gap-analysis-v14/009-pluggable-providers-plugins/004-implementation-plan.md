# 009 — Implementation Plan

## Architecture (ASCII)

```
   ┌───────────────────────────────────────────────────────────────────┐
   │                edgecrab-sdk-core (public, stable)                 │
   │                                                                   │
   │   pub trait LLMProvider { ... }                                   │
   │   pub trait ToolHandler { ... }                                   │
   │   pub trait Plugin {                                              │
   │       fn name(&self) -> &str;                                     │
   │       fn providers(&self) -> Vec<Arc<dyn LLMProvider>>;           │
   │       fn tool_override(                                           │
   │           &self, name: &str,                                      │
   │           original: Arc<dyn ToolHandler>,                         │
   │       ) -> Option<Arc<dyn ToolHandler>>;                          │
   │       fn on_event(&self, e: PluginEvent, ctx: &PluginContext);    │
   │   }                                                               │
   │   pub struct PluginContext {                                      │
   │       pub llm: Arc<dyn LLMHandle>,  // ←  ctx.llm                 │
   │       pub session_id: SessionId,                                  │
   │       pub config: Arc<AppConfig>,                                 │
   │   }                                                               │
   │   pub trait LLMHandle {                                           │
   │       async fn complete(&self, prompt: &str) -> Result<String>;   │
   │       async fn complete_streaming(...) -> Result<Stream>;         │
   │   }                                                               │
   └───────────────────────────────────────────────────────────────────┘
                                  ▲
                                  │ depends
   ┌───────────────────────────────────────────────────────────────────┐
   │                       edgecrab-plugins                            │
   │                                                                   │
   │   PluginHost {                                                    │
   │     discover_native()  ← libloading scans ~/.edgecrab/plugins/    │
   │     discover_wasm()    ← existing                                 │
   │     register_with(agent_builder)                                  │
   │   }                                                               │
   │                                                                   │
   │   ToolOverrideChain (Arc<dyn ToolHandler> → wrapped)              │
   │   ProviderRegistry (alias → Arc<dyn LLMProvider>)                 │
   └───────────────────────────────────────────────────────────────────┘
                                  ▲
                                  │
   ┌───────────────────────────────────────────────────────────────────┐
   │                       edgecrab-core                               │
   │                                                                   │
   │   AgentBuilder::plugins(Arc<PluginHost>)                          │
   │     - consults provider registry before model_catalog             │
   │     - wraps tool handlers via ToolOverrideChain                   │
   │     - passes PluginContext (with LLMHandle) to plugin events      │
   └───────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **Stabilise** | `crates/edgecrab-sdk-core/src/llm.rs` — re-export `LLMProvider`, `LLMHandle` (NEW trait), `ChatResponse`, `StreamEvent` |
| **Stabilise** | `crates/edgecrab-sdk-core/src/plugin.rs` — `Plugin` trait, `PluginContext`, `PluginEvent` enum |
| **Stabilise** | `crates/edgecrab-sdk-core/src/tool.rs` — re-export `ToolHandler`, `ToolContext` |
| **New** | `crates/edgecrab-plugins/src/native.rs` — `libloading`-based loader for `~/.edgecrab/plugins/*.{dylib,so,dll}` |
| **New** | `crates/edgecrab-plugins/src/provider_registry.rs` — alias → `Arc<dyn LLMProvider>` |
| **New** | `crates/edgecrab-plugins/src/tool_override.rs` — chain that wraps handlers |
| **New** | `crates/edgecrab-plugins/src/llm_handle.rs` — bridges `LLMHandle` calls to the active agent's provider |
| **Modify** | `crates/edgecrab-core/src/model_router.rs` — consult plugin provider registry first |
| **Modify** | `crates/edgecrab-tools/src/registry.rs` — `ToolRegistry::dispatch` consults override chain |
| **Modify** | `crates/edgecrab-core/src/agent.rs` — `AgentBuilder::plugins(...)`; on build, wire registries |
| **SDK example** | `sdks/rust/examples/custom_provider/` — full working third-party provider crate |
| **SDK macros** | `crates/edgecrab-sdk-macros/` — `#[edgecrab::plugin]` proc macro for boilerplate |
| **CLI** | `edgecrab plugins install <path|name>` + existing `/plugins` slash command surfaces native plugins |
| **Tests** | a fixture plugin (`tests/fixtures/test_plugin/`) compiled and loaded in CI |

## DRY / SOLID Notes

- **DIP:** `edgecrab-core` depends on `edgecrab-sdk-core` *trait*
  definitions, not on plugin implementations. Plugin crate depends on
  sdk-core, not on `edgecrab-core` directly.
- **OCP:** new plugin event types (`OnTurnStart`, `OnToolCall`,
  `OnAssistantMessage`) extend `PluginEvent` enum; existing plugins that
  don't match those variants simply ignore them.
- **ISP:** `Plugin` is a *small* trait with sensible defaults; plugins
  override only the hooks they care about.
- **SRP:** `PluginHost` does discovery; `ProviderRegistry` does lookup;
  `ToolOverrideChain` does wrapping. Each in its own module.
- **DRY:** the existing tool registration via `inventory!` continues to
  work for first-party tools; override chain is purely additive.

## Trust & Safety

- **Native plugins run in-process** with full host privileges. The CLI
  refuses to load a native plugin unless the user has run
  `edgecrab plugins trust <name>` or the file lives in a "trusted"
  directory (`~/.edgecrab/plugins/trusted/`).
- **WASM and Lua plugins remain sandboxed** as today.
- Plugin file paths logged on load; integrity check via SHA-256 stored
  in `~/.edgecrab/plugins/trust.json`.

## ABI Stability

Use the `stable_abi` crate (or a similar approach: pure C ABI exported
functions returning opaque trait objects) to allow plugins compiled with
a different rustc minor version to load. Document the policy:
*"plugins built against `edgecrab-sdk-core = "x.y"` are guaranteed to
load against host `>=x.y, <x.(y+10)`"*.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Test case: [../014-web-search-backends/](../014-web-search-backends/) — first real `tool_override` consumer.
- Sibling hook: [../030-transform-llm-output-hook/](../030-transform-llm-output-hook/).
