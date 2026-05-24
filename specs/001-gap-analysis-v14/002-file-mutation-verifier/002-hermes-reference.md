# 002 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| File-op interceptor | `hermes-agent/tools/file_operations.py` (records writes/patches/deletes to a per-turn buffer) |
| Verifier rendering | `hermes-agent/agent/display.py` (renders the footer block) |
| Per-turn lifecycle | `hermes-agent/run_agent.py` — buffer is created on user-message receipt, flushed after final assistant message |
| Integration with checkpoints | `hermes-agent/tools/checkpoint_manager.py` — checkpoint snapshot uses same buffer |

## Mechanism

1. Each filesystem-mutating tool (`write_file`, `patch_file`, `delete_file`)
   pushes a `MutationRecord { path, kind, lines_added, lines_removed }`
   onto a per-turn `MutationBuffer`.
2. After the ReAct loop emits its final assistant message, the buffer is
   rendered as a footer block (TTY) **and** appended as a system note to
   the message history (so the next turn's model sees it).
3. Empty buffers produce no footer (zero noise on read-only turns).

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
