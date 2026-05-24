# 009 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| `ProviderProfile` ABC | `hermes-agent/hermes_cli/profiles.py` (or `providers.py`) — abstract `class ProviderProfile` with `chat`, `chat_stream`, `count_tokens`, `pricing` methods + class-level `aliases` |
| Plugin host | `hermes-agent/plugins/` package (`__init__.py` discovery via entry-point group) |
| Tool override hook | `def tool_override(name: str, original: Callable) -> Callable | None` |
| `ctx.llm` | `PluginContext` passed to every hook carries an LLM handle resolved against the *active* agent's provider |
| Profile distribution | `pip install` of any package that exposes a `hermes.providers` entry point auto-registers a profile |

## Plugin Interface (sketch)

```python
# In a third-party package:
class MyCorpProvider(ProviderProfile):
    aliases = ("mycorp/turbo-v9",)
    async def chat(self, model, messages, tools): ...
    async def chat_stream(self, model, messages, tools): ...
    def count_tokens(self, messages): ...
    def pricing(self, model): ...

# entry_points = { "hermes.providers": ["mycorp = mycorp.provider:MyCorpProvider"] }
```

```python
# Tool override:
def tool_override(name, original):
    if name == "web_search":
        async def patched(args, ctx):
            # use ctx.llm to rewrite the query first
            improved = await ctx.llm.complete("Rewrite for better recall: " + args["query"])
            return await original({"query": improved.text, **args}, ctx)
        return patched
    return None
```

## Lifecycle

1. Startup: discover entry points → register profiles + plugin hooks.
2. Per turn: `LLMRouter.resolve(model_alias)` consults registered
   profiles before built-ins (allowing overrides).
3. Per tool dispatch: registry consults `tool_override` chain before
   invoking the built-in handler.
4. Plugin code runs in-process with full Python privileges (this is a
   *trust* model — for sandboxed plugins use the WASM path EdgeCrab
   already has).

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
