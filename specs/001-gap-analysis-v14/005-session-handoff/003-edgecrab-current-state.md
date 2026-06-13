# 005 — EdgeCrab Current State

## `/handoff <platform>` — CLI → gateway (implemented)

| Layer | Module |
|-------|--------|
| CLI handler | `crates/edgecrab-cli/src/app.rs` → `handle_session_handoff` |
| Command | `commands.rs` → `SessionHandoff` (CLI only; not gateway-visible) |
| State machine | `edgecrab-state/session_db.rs` → `handoff_state` columns + API |
| Gateway watcher | `edgecrab-gateway/platform_handoff.rs` → `SessionHandoffWatcher` |
| Session rebind | `edgecrab-gateway/session.rs` → `rebind_cli_session` |
| Synthetic message | `edgecrab-core/session_handoff.rs` |
| Thread hook | `PlatformAdapter::create_handoff_thread` — Telegram (`createForumTopic`), Discord (channel thread + seed fallback), Slack (seed `thread_ts`) |

## `/transfer-model <provider/model>` — model transfer (implemented)

| Layer | Module |
|-------|--------|
| Orchestrator | `edgecrab-core/model_transfer.rs` |
| Agent API | `Agent::perform_model_transfer` |
| Persistence | `model_transfers` table + `/insights` |
| CLI + gateway | `/model` and `/transfer-model` → `perform_model_transfer` |

## Remaining gaps vs Hermes

- **Profile/OAuth pool rotation** on model transfer (tracked in spec 024-oauth-providers; `create_provider_for_model` is already the single factory).

## Cross-References

- [proof/implementation-proof.md](proof/implementation-proof.md)
