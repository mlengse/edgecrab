# 021 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │           edgecrab-core/src/curator/ (NEW module)                │
   │                                                                  │
   │   mod.rs       — Curator struct + entrypoint                     │
   │   trigger.rs   — write counter + time-based decision             │
   │   prompt.rs    — system prompt template for curator agent        │
   │   apply.rs     — atomic rewrite + archive move + diff log        │
   │                                                                  │
   │   async fn maybe_run(state: &CuratorState, agent_builder: ...)  │
   │       if trigger.should_run() {                                  │
   │           let plan = run_curator_subagent(memory_text).await?;   │
   │           apply::apply(plan)?;                                   │
   │           record_diff();                                         │
   │       }                                                          │
   └──────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌──────────────────────────────────────────────────────────────────┐
   │           edgecrab-tools/src/tools/memory.rs (hook)              │
   │                                                                  │
   │   on successful memory write: increment trigger counter;         │
   │     spawn curator::maybe_run as detached task (non-blocking).    │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **Module** | `crates/edgecrab-core/src/curator/{mod,trigger,prompt,apply}.rs` |
| **State file** | `~/.edgecrab/curator/state.json` (last_run_ts, writes_since_last_run) |
| **Diff log** | `~/.edgecrab/curator/runs/<ts>.json` (input hash, plan, archive list) |
| **Archive dir** | `~/.edgecrab/memories/archive/MEMORY-archive-<ts>.md` |
| **Curator model** | `config.curator.model` (default: cheap model from catalog) |
| **Trigger thresholds** | `config.curator.writes_between_runs: 20`, `config.curator.min_interval_secs: 86400` |
| **Slash command** | `/curator run` (force), `/curator status`, `/curator revert [N]` |
| **Subagent** | use existing `sub_agent_runner.rs`; system prompt instructs to output a strict JSON plan |
| **Atomic apply** | write rewritten MEMORY.md.tmp → fsync → rename; same for archive |
| **Lock** | curator runs under a file lock to prevent concurrent runs |

## Plan JSON Schema

```
{
  "kept": [{"id":"…","text":"…"}, ...],
  "merged": [{"new_text":"…","source_ids":["…","…"]}],
  "archived": [{"id":"…","reason":"stale|duplicate|contradicted"}]
}
```

## DRY / SOLID Notes

- **SRP:** trigger, prompt, apply are separate modules.
- **DIP:** depends on `AgentBuilder` abstraction → curator can use any
  provider.
- **DRY:** atomic-rewrite helper from memory.rs reused.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
