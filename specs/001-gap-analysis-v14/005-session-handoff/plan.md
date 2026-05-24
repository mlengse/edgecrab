# 005 — Session Handoff Implementation Plan

## Scope

Implement `/handoff <provider/model>` as an explicit, user-visible model transfer that
preserves conversation history, persistent goals, and todos while:

1. Generating a one-paragraph in-flight task brief (auxiliary LLM + structural fallback)
2. Auto-compressing when the target model has a smaller context window
3. Rebuilding the cached system prompt for the new provider (cache-safe per 004)
4. Recording handoff events in SQLite for `/insights`
5. Surfacing the command in CLI + gateway (Telegram, Slack, Discord, …)

## Architecture

```
/handoff copilot/gpt-5-mini
     │
     ▼
HandoffOrchestrator (edgecrab-core/handoff.rs)
  1. resolve_handoff_target()     → ModelCatalog lookup (fail if unknown)
  2. create_target_provider()     → auth probe (fail before mutation)
  3. maybe_compress_for_target()  → compress_with_llm if over threshold
  4. generate_handoff_brief()     → auxiliary LLM (copilot/gpt-5-mini default)
  5. agent.swap_model()           → hot-swap provider + model string
  6. agent.invalidate_system_prompt()
  7. push synthetic user message
  8. session_db.record_handoff()
  9. StreamEvent::HandoffComplete
```

## Files

| Action | Path |
|--------|------|
| New | `crates/edgecrab-core/src/handoff.rs` |
| Export | `crates/edgecrab-core/src/lib.rs` |
| Agent API | `crates/edgecrab-core/src/agent.rs` — `perform_handling_handoff`, `StreamEvent::HandoffComplete` |
| DB schema v9 | `crates/edgecrab-state/src/session_db.rs`, `schema.sql` |
| CLI command | `crates/edgecrab-cli/src/commands.rs`, `app.rs` |
| Gateway | `crates/edgecrab-gateway/src/run.rs` |
| Catalog | `crates/edgecrab-command-catalog/src/lib.rs` |
| Proof | `specs/001-gap-analysis-v14/005-session-handoff/proof/implementation-proof.md` |

## Tests (≥6 in `handoff::`)

1. Unknown target model → error, no mutation
2. Smaller window + compress OK
3. Smaller window + compress fail → refuse swap
4. Brief generation OK (MockProvider)
5. Brief fallback when LLM fails
6. Provider creation failure leaves snapshot unchanged

## Hermes parity note

Hermes `/handoff <platform>` is cross-platform session transfer (CLI → Telegram).
This feature is **model/profile handoff with brief** — closer to Hermes fallback
provider swap + title_generator auxiliary call. EdgeCrab exceeds Hermes on
transparency (brief, window check, compression notice, insights history).
