# 020 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │             edgecrab-state/src/last_session.rs (NEW)             │
   │                                                                  │
   │   pub fn read_pointer() -> Option<Uuid>                          │
   │   pub fn write_pointer(id: Uuid) -> Result<()>                   │
   │       (atomic: write to .tmp, rename)                            │
   │   pub fn clear_pointer()                                         │
   └──────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌──────────────────────────────────────────────────────────────────┐
   │             edgecrab-cli/src/setup.rs (extend)                   │
   │                                                                  │
   │   fn decide_session(cfg: &Config, db: &SessionDb) -> Action {    │
   │       let mode = cfg.session.auto_resume;  // prompt|always|never│
   │       if mode == Never { return New; }                           │
   │       let id = match read_pointer() { Some(i) => i, None=>      │
   │            return New };                                         │
   │       let row = db.get_session_meta(id)?;                        │
   │       let age = now - row.updated_at;                            │
   │       if age > cfg.session.auto_resume_max_age_secs              │
   │            { return New; }                                       │
   │       match mode {                                               │
   │           Always => Resume(id),                                  │
   │           Prompt => prompt_user(id, age, row.message_count),     │
   │           Never  => unreachable!(),                              │
   │       }                                                          │
   │   }                                                              │
   │                                                                  │
   │   on every assistant turn: write_pointer(current_session.id)     │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **Pointer module** | `crates/edgecrab-state/src/last_session.rs` |
| **`get_session_meta` query** | new query in `crates/edgecrab-state/src/` returning `{updated_at, message_count, title}` |
| **Launch decision** | `crates/edgecrab-cli/src/setup.rs` `decide_session` helper |
| **Prompt UI** | small ratatui inline prompt before main loop OR plain `dialoguer`-style stdin prompt in non-TUI launch |
| **Update on activity** | hook in `App::push_message` or equivalent — after each user/assistant message, refresh pointer |
| **Config** | `session.auto_resume: "prompt"` (default), `session.auto_resume_max_age_secs: 7200`, `session.auto_resume_max_messages: 200` (refuse to resume sessions over 200 msgs to avoid massive context reload — start fresh suggestion) |
| **CLI flag overrides** | `--resume` (force resume even outside window), `--new` (force new) |
| **Subcommand surface** | `edgecrab` (default), `edgecrab resume`, `edgecrab new` |
| **Tests** | unit tests on `decide_session` matrix; integration test creating a temp `EDGECRAB_HOME` |

## Edge Cases

- Pointer file references deleted session → ignore, start new, clear
  pointer.
- Pointer file unreadable / corrupt → log warn, start new.
- Multiple concurrent `edgecrab` launches → pointer file might race;
  use atomic rename for write, last-writer-wins is acceptable.
- ACP/gateway entrypoints do NOT use auto-resume (they manage their
  own session keys per platform user).

## DRY / SOLID Notes

- **SRP:** pointer IO in state crate; decision in CLI; UI is small.
- **OCP:** add `--mode` CLI flags without touching the decision
  function.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
