# 005 — Implementation Plan

## Architecture (ASCII)

```
   /handoff anthropic/claude-haiku-4
        │
        ▼
   ┌──────────────────────────────────────────────────────┐
   │  HandoffOrchestrator (edgecrab-core)                 │
   │                                                      │
   │  1. resolve_target(model_str)  -> ModelCatalogEntry  │
   │  2. check_window(history, target.ctx_window)         │
   │       if too big → compress_with_llm(...)            │
   │  3. summarise_in_flight(history) -> HandoffBrief     │
   │       (auxiliary 1-paragraph LLM call, cheapest      │
   │        model on current provider)                    │
   │  4. agent.set_provider(target.provider)              │
   │     agent.set_model(target.model)                    │
   │  5. push synthetic user message:                     │
   │        "Continuing from previous session: <brief>"   │
   │  6. emit StreamEvent::HandoffComplete                │
   │  7. session_db: record handoff event                 │
   └──────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **New module** | `crates/edgecrab-core/src/handoff.rs` — `HandoffOrchestrator`, `HandoffBrief` |
| **Slash command** | `crates/edgecrab-cli/src/commands.rs` — `CommandResult::Handoff { target: String }` |
| **Gateway dispatch** | `crates/edgecrab-gateway/src/run.rs` — same |
| **Stream event** | `crates/edgecrab-core/src/agent.rs` — `StreamEvent::HandoffComplete { from, to, brief }` |
| **Session DB schema** | new `handoffs` table (`session_id`, `from_model`, `to_model`, `brief`, `ts`) |
| **Insights** | `crates/edgecrab-core/src/insights.rs` (if exists; else `/cost`) surfaces handoff history |

## DRY / SOLID Notes

- **DRY:** auxiliary-call wrapper reused from compression/title-generator
  if EdgeCrab has one; else introduce `AuxiliaryClient` trait now.
- **SRP:** `HandoffOrchestrator` orchestrates only; window check uses
  existing `model_catalog`; compression uses existing
  `compression::compress_with_llm`.
- **Cache safety:** the new provider gets a fresh `SystemPromptBlocks`
  built once and cached (per `004-prompt-prefix-cache`).

## Failure Modes

| Failure | Handling |
|---------|----------|
| Target model unknown to catalog | Return error to user, no state change |
| Target model has smaller window AND compression fails | Refuse swap, message user |
| Auxiliary call for brief fails | Fall back to structural summary (last N user/assistant turns concatenated) |
| New provider auth missing | Refuse swap, prompt for auth |

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
- Window check uses `model_catalog`: see `crates/edgecrab-core/src/model_catalog.rs`.
