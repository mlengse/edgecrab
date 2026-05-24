# 005 — Session Handoff — Implementation Proof

**Branch:** `feat/session-handoff`  
**Date:** 2026-05-24  
**Status:** Implemented

## Acceptance Criteria Checklist

| Criterion | Status | Evidence |
|-----------|--------|----------|
| `/handoff <provider/model>` hot-swaps active model | ✅ | `Agent::perform_handoff`, CLI `handle_handoff`, gateway `handle_handoff_command` |
| User sees one-paragraph in-flight task brief | ✅ | `generate_handoff_brief` + synthetic user message + `StreamEvent::HandoffComplete` / TUI notice |
| Persistent goals survive handoff | ✅ | Goals live in SQLite (`session_goals`); handoff never touches goal tables |
| History intact; compression when window forces it | ✅ | `maybe_compress_for_handoff`; user notified via `compressed` flag in outcome |
| Auth failure leaves original model | ✅ | `create_target_provider` runs before any session mutation |
| Gateway `/handoff` (Telegram, Slack, Discord, …) | ✅ | `run.rs` dispatch + `SlashCommandSpec` gateway=true |
| `/insights` lists session handoffs | ✅ | `SessionDb::list_handoffs` wired in CLI + gateway insights formatters |
| `cargo clippy --workspace -- -D warnings` | ✅ | Clean (2026-05-24 run) |
| ≥6 `handoff::` tests | ✅ | 9 unit tests in `handoff.rs` + DB test + command dispatch test |
| No mutation of cached system prompt mid-handoff | ✅ | Prompt cleared (`invalidate` pattern); rebuilt on next turn for new provider |

## Test Results

```bash
cargo test -p edgecrab-core --lib handoff          # 9 passed
cargo test -p edgecrab-state record_and_list_handoffs  # 1 passed
cargo clippy --workspace -- -D warnings          # clean
cargo test -p edgecrab-core -p edgecrab-state -p edgecrab-gateway --lib  # passed
```

## Architecture Delivered

```
/handoff copilot/gpt-5-mini
  → HandoffOrchestrator::execute
      resolve_handoff_target (ModelCatalog)
      create_target_provider (auth probe)
      maybe_compress_for_handoff (target window)
      generate_handoff_brief (auxiliary LLM → structural fallback)
  → Agent::swap_model + clear cached prompt blocks
  → synthetic user message with brief
  → SessionDb::record_handoff
  → StreamEvent::HandoffComplete
```

## Files Changed

| File | Change |
|------|--------|
| `crates/edgecrab-core/src/handoff.rs` | **New** orchestrator + brief + window check |
| `crates/edgecrab-core/src/agent.rs` | `perform_handoff`, `StreamEvent::HandoffComplete` |
| `crates/edgecrab-state/src/session_db.rs` | Schema v9 `handoffs` table |
| `crates/edgecrab-cli/src/commands.rs` | `/handoff` command |
| `crates/edgecrab-cli/src/app.rs` | TUI handler + insights section |
| `crates/edgecrab-gateway/src/run.rs` | Gateway handler + insights |
| `crates/edgecrab-command-catalog/src/lib.rs` | Catalog entry |

---

## Brutal Honest Assessment vs Nous Hermes Agent

### What Hermes Actually Has (code is law)

After reading `/Users/raphaelmansuy/Github/03-working/hermes-agent`:

1. **`/handoff <platform>`** (CLI → gateway) — cross-platform session transfer with thread creation, full transcript replay, synthetic confirmation turn. Implemented in `cli.py`, `hermes_state.py`, `tests/hermes_cli/test_session_handoff.py`. **This is NOT model handoff.**

2. **Model swap** — Hermes has `/model` hot-swap and **fallback provider** activation (`try_activate_fallback` in `chat_completion_helpers.py`) that swaps model+provider mid-session on failure. No user-visible brief, no explicit window check, no handoff history in insights.

3. **Handoff brief pattern** — Hermes uses "handoff summary" language in **context compression** (`context_compressor.py`) and Kanban worker tools, not as a first-class `/handoff model` command.

4. **Release note v0.14.0** mentions live model/persona handoff — the closest production behavior is fallback swap + compression summaries, not a dedicated slash command matching our spec.

### EdgeCrab vs Hermes (this feature)

| Dimension | Hermes | EdgeCrab 005 |
|-----------|--------|--------------|
| Explicit `/handoff <model>` command | ❌ Not found in codebase | ✅ |
| In-flight task brief before next turn | ⚠️ Implicit via compression only | ✅ Dedicated auxiliary call + fallback |
| Context window check before swap | ❌ Silent | ✅ Auto-compress or refuse |
| User confirmation of what was preserved | ❌ | ✅ Brief + compression notice |
| Handoff history in insights | ❌ | ✅ SQLite `handoffs` table |
| Cross-platform CLI→Telegram handoff | ✅ Mature | ❌ Out of scope (different feature) |
| Profile / OAuth pool swap | ✅ Via fallback chain | ⚠️ Partial — same as `/model` provider factory; no multi-key pool rotation |

### Verdict

**For model/profile live transfer with transparency:** EdgeCrab **meets and exceeds** the Hermes equivalent. Hermes does not implement this as a named, user-facing workflow; EdgeCrab adds brief generation, window safety, cache-safe prompt invalidation, and auditable history.

**For cross-platform session handoff (CLI → Telegram):** Hermes **exceeds** EdgeCrab — that remains a separate gap (Hermes `/handoff telegram`).

**Rust-specific advantages:** Type-safe `HandoffError` enum, compile-time `StreamEvent` exhaustiveness, SQLite migration v9 with WAL — appropriate for EdgeCrab's architecture.

### Known Gaps / Follow-ups

- No interactive E2E with live `copilot/gpt-5-mini` in CI (requires Copilot credentials; unit tests use `MockProvider`).
- Profile distribution / multi-OAuth-pool handoff not implemented (Hermes profiles.py depth not ported).
- `/handoff` does not persist default model to config on gateway (CLI does via `persist_model_to_config`).

### Recommendation

Ship 005 as **model handoff parity+**. Track cross-platform `/handoff <platform>` as a separate spec if gateway UX parity with Hermes CLI is required.
