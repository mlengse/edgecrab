# 009 — Acceptance Criteria

## Functional

- [ ] A third-party crate implementing `Plugin` and providing a new
      `LLMProvider` can be loaded at runtime via
      `edgecrab plugins install`, and `/model my-provider/foo` uses it.
- [ ] `tool_override` can wrap `web_search` so that all calls are routed
      through the wrapper (asserted by a counter inside the plugin).
- [ ] Inside a plugin event handler, `ctx.llm.complete("hello")` returns
      a non-empty string and consumes tokens reported in `/cost`.
- [ ] First-party (inventory-registered) tools continue to work unchanged.
- [ ] WASM and Lua plugin paths still load.

## Trust

- [ ] Loading an untrusted native plugin requires
      `edgecrab plugins trust <name>` first; otherwise the host refuses
      with a clear error.
- [ ] SHA-256 mismatch on a trusted plugin file → refuse load + warn.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] `cargo test -p edgecrab-plugins` includes a CI step that compiles
      `tests/fixtures/test_plugin/` and loads it dynamically.
- [ ] Public API in `edgecrab-sdk-core` is `#[non_exhaustive]` where
      reasonable to allow growth without breakage.
- [ ] No `unwrap()` in plugin host code paths.

## Documentation

- [ ] `sdks/rust/examples/custom_provider/` is a complete, copy-able
      example with README.
- [ ] `AGENTS.md` gains a "Native Plugins" subsection (alongside WASM/Lua).
- [ ] ABI-stability policy written into the SDK README.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
