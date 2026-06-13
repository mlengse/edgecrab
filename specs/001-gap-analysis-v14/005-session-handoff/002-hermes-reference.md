# 005 — Hermes Reference

## A. Cross-platform `/handoff <platform>` (this spec's primary target)

| Concern | Hermes file |
|---------|-------------|
| CLI command | `hermes-agent/cli.py` → `_handle_handoff_command` |
| State machine | `hermes-agent/hermes_state.py` → `request_handoff`, `claim_handoff`, `complete_handoff`, `fail_handoff` |
| Gateway watcher | `hermes-agent/gateway/run.py` → `_handoff_watcher`, `_process_handoff` |
| Session rebind | `hermes-agent/gateway/session.py` → `switch_session` |
| Thread creation | `gateway/platforms/telegram.py`, `slack.py`, `plugins/platforms/discord/adapter.py` → `create_handoff_thread` |
| Docs | `hermes-agent/website/docs/user-guide/sessions.md` (Cross-Platform Handoff) |
| Tests | `hermes-agent/tests/hermes_cli/test_session_handoff.py` |

### Mechanism (Hermes)

```
/handoff telegram
   1. Validate platform + home channel (/sethome).
   2. Reject mid-turn (agent running).
   3. UPDATE sessions SET handoff_state='pending', handoff_platform='telegram'.
   4. CLI poll-blocks on terminal state (60s timeout).
   5. Gateway watcher: pending → running → rebind session_key → synthetic user turn → adapter.send.
   6. CLI exits on completed; /resume later.
```

## B. Model swap (NOT called `/handoff` in Hermes)

Hermes has `/model` hot-swap and **fallback provider** activation — no user-visible brief, no `/handoff model` command. EdgeCrab's **`/transfer-model`** is an intentional enhancement documented separately in this folder.

| Concern | Hermes file |
|---------|-------------|
| Model swap | `hermes_cli/model_switch.py`, `agent/agent_runtime_helpers.py` |
| Compression summary | `agent/context_compressor.py` (implicit handoff language) |

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
