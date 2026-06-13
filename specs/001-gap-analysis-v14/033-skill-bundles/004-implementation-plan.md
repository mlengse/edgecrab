# 033 — Implementation Plan

## Strategy

A bundle is a thin index that references existing skills by name. Resolve
member names through the existing `SkillsManifest`, then reuse
`load_skill_summary`'s body-reading path to inject each member. No new
skill storage, no duplication.

## Architecture

```
   user: /release        (or /skills bundle load release)
            │
            ▼
   ┌────────────────────────────────────────────────┐
   │ plugins.rs  /skills bundle dispatcher (NEW)      │
   └────────────────────────────────────────────────┘
            │ resolve "release"
            ▼
   ┌────────────────────────────────────────────────┐
   │ skills/bundles.yaml         (NEW manifest)       │
   │   release:                                       │
   │     - changelog-writer                           │
   │     - semver                                     │
   │     - git-tag                                    │
   └────────────────────────────────────────────────┘
            │ member names
            ▼
   ┌────────────────────────────────────────────────┐
   │ SkillsManifest::build  (REUSE) → resolve paths   │
   │ load_skill_summary per member (REUSE) → bodies   │
   └────────────────────────────────────────────────┘
            │
            ▼
   inject all member skill bodies into active context in one turn
   (missing member → warn + skip, never silently load partial)
```

## File Map

| File | Change |
|------|--------|
| `~/.edgecrab/skills/bundles.yaml` | **NEW** user data — `name: [member skills]` |
| [crates/edgecrab-core/src/prompt_builder.rs](../../../crates/edgecrab-core/src/prompt_builder.rs#L1926) | add `load_bundle(name)` — resolve members via `SkillsManifest`, reuse `load_skill_summary` body reader |
| [crates/edgecrab-cli/src/plugins.rs](../../../crates/edgecrab-cli/src/plugins.rs) | `/skills bundle list/load/show`; register `/<bundle-name>` shortcut |
| [crates/edgecrab-cli/src/commands.rs](../../../crates/edgecrab-cli/src/commands.rs) | `CommandResult::LoadSkillBundle(name)` variant + dispatch |
| [crates/edgecrab-gateway/src/run.rs](../../../crates/edgecrab-gateway/src/run.rs) | mirror `/skills bundle load` for gateway parity |

## DRY / SOLID Notes

- **DRY:** bundles hold *references*, not skill copies — editing a member
  skill updates every bundle that includes it.
- **SRP:** the manifest declares membership; the resolver loads; the
  command dispatches. Three concerns, three seams.
- **Reuse:** `SkillsManifest::build` + `load_skill_summary` already do the
  enumeration and body-reading — `load_bundle` is glue, not new I/O.
- **Fail loud:** a missing member warns and is skipped; the user is told
  the bundle loaded N of M, never a silent partial.

## Sequencing

1. `bundles.yaml` schema + parser.
2. `load_bundle` resolver in `prompt_builder` (reusing existing readers).
3. `/skills bundle …` + `/<name>` dispatch in CLI.
4. Gateway parity.

## Cross-References

- [003-edgecrab-current-state.md](003-edgecrab-current-state.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
