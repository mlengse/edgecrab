# 009 — Pluggable `ProviderProfile` ABC + Plugin `tool_override` + `ctx.llm`

**Tier:** A | **Impact:** 5 | **Value-per-Effort:** 3 | **Risk:** 4
**Primitive moved:** Reliability (extensibility unlocks community fixes)

## Why It Matters (First Principles)

The fastest agent platforms are not the ones with the most features built-in
— they are the ones that let users **add their own provider, tool, or
behaviour without forking the core**. Hermes v0.13/v0.14 made three
parallel investments here:

1. **`ProviderProfile` ABC** — a Python abstract base class that any
   third-party can subclass to add a new LLM provider (different auth,
   different wire format, different streaming, different rate-limit
   behaviour) without touching `hermes-agent`.
2. **Plugin `tool_override` hook** — plugins can replace or wrap a
   built-in tool's behaviour (e.g. swap `web_search` to use SearXNG
   without forking `web_tools.py`).
3. **`ctx.llm`** — plugins receive a handle to call the active LLM
   directly, so a plugin can do its own reasoning, summarisation, or
   classification calls without re-implementing provider auth.

EdgeCrab today has the WASM/Lua plugin crate (`edgecrab-plugins`) but
none of the three hooks. This is the difference between a closed
product and a platform.

## The Gap

| Hermes capability | EdgeCrab status |
|-------------------|-----------------|
| Subclass-able provider profile | No public trait stable for external impls; `LLMProvider` exists internally but not exposed via SDK |
| Override built-in tool from plugin | No; plugin can add tools, not replace |
| Plugin handle on the live LLM (`ctx.llm`) | No; plugin sandbox cannot call the provider |

## What EdgeCrab Gets Wrong Today

`edgecrab-plugins` is documented in `AGENTS.md` but the only extension
point is "add a new tool via inventory!" — which is **compile-time** and
requires recompiling EdgeCrab. Hermes' Python hooks are runtime and
distributable via `pip install hermes-plugin-mycorp`. EdgeCrab needs a
similar runtime-loadable plugin surface with three explicit hooks.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Related: [030-transform-llm-output-hook/](../030-transform-llm-output-hook/) (sibling plugin hook)
- Related: [014-web-search-backends/](../014-web-search-backends/) (a clean test case for `tool_override`)
