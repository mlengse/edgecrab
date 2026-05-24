# 005 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| Slash command | `hermes-agent/hermes_cli/commands.py` (no direct entry — wired via plugin or `run_agent.py`) |
| Provider/profile swap | `hermes-agent/hermes_cli/profiles.py` + `hermes_cli/providers.py` |
| Handoff brief generation | `hermes-agent/agent/title_generator.py`-style auxiliary call producing 1-paragraph "current task" summary |
| Cache migration | `hermes-agent/agent/prompt_caching.py` re-emits the stable block for the new provider |

## Mechanism

```
/handoff anthropic/claude-haiku-4
   1. Auxiliary LLM call: "Summarise the in-flight task in 1 paragraph."
   2. Validate target model context window ≥ history size; else auto-compress.
   3. Rebuild PromptBuilder against the new provider's adapter.
   4. Send the handoff brief as the first message of the new provider session.
   5. Drop the old provider connection.
   6. Persist the swap in session metadata.
```

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
