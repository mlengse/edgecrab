# 005 — Implementation Plan

## Feature A: `/handoff <platform>` (Hermes parity)

```
CLI: /handoff telegram
  → validate home channel (AppConfig.gateway.*.home_channel)
  → SessionDb::request_session_handoff(session_id, platform)
  → poll get_session_handoff_status (60s)
  → on completed: exit CLI + /resume hint

Gateway: SessionHandoffWatcher (2s poll)
  → list_pending_session_handoffs
  → claim_session_handoff (pending → running)
  → resolve adapter + home channel
  → create_handoff_thread (optional)
  → SessionManager::rebind_cli_session(key, cli_session_id)
  → rebind_session_routing in SQLite
  → agent.chat(synthetic_message) + adapter.send
  → complete_session_handoff | fail_session_handoff
```

**Schema v10:** `sessions.handoff_state`, `handoff_platform`, `handoff_error`

## Feature B: `/transfer-model <provider/model>` (renamed from old `/handoff`)

```
/transfer-model copilot/gpt-5-mini
  → ModelTransferOrchestrator::execute
  → Agent::perform_model_transfer
  → SessionDb::record_model_transfer
  → StreamEvent::ModelTransferComplete
```

**Schema v9/v10:** `model_transfers` table (renamed from `handoffs`)

## SOLID boundaries

| Type | Responsibility |
|------|----------------|
| `SessionHandoffWatcher` | Platform transfer only |
| `ModelTransferOrchestrator` | Model pipeline only |
| `SessionState::apply_model_transfer_outcome` | Post-transfer session mutation |
| `SessionDb` | Both state machines + audit tables |

## Cross-References

- [005-acceptance-criteria.md](005-acceptance-criteria.md) · [proof/implementation-proof.md](proof/implementation-proof.md)
