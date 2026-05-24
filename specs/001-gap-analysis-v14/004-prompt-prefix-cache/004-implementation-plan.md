# 004 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────┐
   │                  edgecrab-core/prompt_builder                │
   │                                                              │
   │   build() now returns SystemPromptBlocks {                   │
   │     stable: String,    ← identity, soul.md, agents.md,       │
   │                          skills index, memory guidance       │
   │     volatile: String,  ← date/time, per-session context      │
   │   }                                                          │
   └─────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
   ┌──────────────────────────────────────────────────────────────┐
   │              Anthropic provider adapter                      │
   │                                                              │
   │   system = [                                                 │
   │     { type: text, text: stable,                              │
   │       cache_control: { type: ephemeral, ttl: "1h" } },       │
   │     { type: text, text: volatile }                           │
   │   ]                                                          │
   └──────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **Refactor** | `crates/edgecrab-core/src/prompt_builder.rs` — `PromptBuilder::build()` returns `SystemPromptBlocks { stable, volatile }` instead of `String`. |
| **Compat shim** | Existing `cached_system_prompt: String` becomes two fields. |
| **Anthropic adapter** | Whichever crate hosts the Anthropic client — attach `cache_control` block on the stable segment only. Add `extra_headers: anthropic-beta: prompt-caching-2024-07-31, extended-cache-ttl-2025-04-11` (1h cache requires beta flag). |
| **Cost telemetry** | `crates/edgecrab-core/src/pricing.rs` — surface `cache_read_input_tokens` and `cache_creation_input_tokens` in `/cost` output. |
| **Goal/footer injection sites** | Confirmed to operate on the `messages: Vec<Message>` (NOT on the system prompt) so they remain cache-safe. |
| **Config** | `cache.prompt_prefix.enabled: true`, `cache.prompt_prefix.ttl: "1h"` (or `"5m"`). |

## DRY / SOLID Notes

- **SRP:** `PromptBuilder` produces blocks; the adapter decides how to
  serialise them into provider-specific cache control.
- **OCP:** non-Anthropic providers ignore `volatile` vs `stable` and just
  concatenate — zero behaviour change for OpenAI etc.
- **Stability invariant:** anything that varies per-session/per-turn
  MUST go into the `volatile` block. Compile-time test: a function
  `stable_block_hash(builder)` that asserts the hash is identical when
  only `now()`, `session_id`, or `cwd` change.

## Risk Notes

- The 1h cache requires the `extended-cache-ttl-2025-04-11` beta header.
  If Anthropic deprecates this, fall back to 5m gracefully.
- If `SOUL.md` / `AGENTS.md` change on disk between sessions, the cached
  prefix correctly invalidates — Anthropic uses content hash, not session id.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Cross-cutting cache safety: [../001-persistent-goals/004-implementation-plan.md](../001-persistent-goals/004-implementation-plan.md), [../002-file-mutation-verifier/004-implementation-plan.md](../002-file-mutation-verifier/004-implementation-plan.md)
