# 033 — Acceptance Criteria

## Bundle manifest

- [ ] `~/.edgecrab/skills/bundles.yaml` parses `name: [member, …]` maps.
- [ ] A bundle referencing a non-existent skill is reported (not a parse
      failure) and that member is skipped at load.

## Loading

- [ ] `/skills bundle load <name>` injects every resolvable member skill
      body into the active context in one command.
- [ ] `/<bundle-name>` is a working shortcut for the above.
- [ ] Loading reports "loaded N of M skills" and names any skipped member.
- [ ] `/skills bundle list` lists bundles; `/skills bundle show <name>`
      lists members.

## DRY guarantees

- [ ] Bundles store **references only** — no skill body is duplicated in
      `bundles.yaml`.
- [ ] Editing a member `SKILL.md` is reflected on the next bundle load
      with no bundle edit (regression test).
- [ ] `load_bundle` reuses `SkillsManifest::build` and `load_skill_summary`
      — no second skill-enumeration implementation exists.

## Parity

- [ ] Gateway exposes `/skills bundle load <name>` with the same behaviour
      as the CLI.

## Non-regression

- [ ] Existing `load_skill_summary_*` tests still pass.
- [ ] `cargo test --workspace` green; `cargo clippy --workspace -- -D warnings` clean.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
