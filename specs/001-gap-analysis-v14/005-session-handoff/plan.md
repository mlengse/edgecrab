# 005 — Session Handoff Plan

## Scope split (2026-05-24)

1. **`/handoff <platform>`** — Hermes cross-platform session transfer (CLI → gateway).
2. **`/transfer-model <provider/model>`** — Renamed from the earlier misnamed `/handoff` model pipeline.

## Deliverables

| Item | Crate | Status |
|------|-------|--------|
| `session_handoff.rs` | edgecrab-core | ✅ constants + synthetic message |
| `platform_handoff.rs` | edgecrab-gateway | ✅ watcher |
| `model_transfer.rs` | edgecrab-core | ✅ renamed orchestrator |
| Schema v10 | edgecrab-state | ✅ handoff columns + `model_transfers` rename |
| CLI handlers | edgecrab-cli | ✅ both commands |
| Command catalog | edgecrab-command-catalog | ✅ split entries |
| Spec redocumentation | specs/005-session-handoff | ✅ |

## Migration note

Existing DBs on schema v9 with `handoffs` table are migrated to `model_transfers` on v10 upgrade.
