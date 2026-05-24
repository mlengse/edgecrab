# 005 — Acceptance Criteria

## `/handoff <platform>` (Hermes parity)

- [x] CLI-only command (not exposed on gateway slash dispatch)
- [x] Validates platform + configured home channel
- [x] Rejects mid-turn (agent busy)
- [x] SQLite state machine: pending → running → completed | failed
- [x] Gateway watcher processes pending rows
- [x] Rebinds destination session to CLI `session_id` + restores transcript
- [x] Synthetic confirmation turn on platform
- [x] CLI exits on success with `/resume` hint
- [x] 60s timeout with failed state + intact CLI session
- [x] Thread creation on Telegram/Discord/Slack (falls back to home channel on failure)

## Model transfer (`/model` + `/transfer-model`)

- [x] Hot-swaps active model with brief + window check
- [x] `/model <provider/model>` uses the same pipeline as `/transfer-model`
- [x] Mid-turn rejection on CLI and gateway for both commands
- [x] Persistent goals survive
- [x] Gateway + CLI support
- [x] `/insights` lists `model_transfers` for session
- [x] `cargo test -p edgecrab-core --lib model_transfer` ≥ 6 tests
- [x] `cargo clippy` clean on affected crates

## Cross-References

- [proof/implementation-proof.md](proof/implementation-proof.md)
