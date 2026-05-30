# 033 — EdgeCrab Current State (Code Is Law)

## What the skill system does today

| Capability | Code path |
|------------|-----------|
| Discover + stat skills | [crates/edgecrab-core/src/prompt_builder.rs](../../../crates/edgecrab-core/src/prompt_builder.rs#L66) `SkillsManifest::build` (one level deep) |
| Inject compact skill index | [crates/edgecrab-core/src/prompt_builder.rs](../../../crates/edgecrab-core/src/prompt_builder.rs#L1926) `load_skill_summary()` |
| Encourage saving skills | [crates/edgecrab-core/src/prompt_builder.rs](../../../crates/edgecrab-core/src/prompt_builder.rs#L657) `SKILLS_GUIDANCE` |
| List / view / install / remove | [crates/edgecrab-cli/src/plugins.rs](../../../crates/edgecrab-cli/src/plugins.rs) `/skills` command (`list`/`view`/`install`/`remove`/`hub`) |
| Install-time bundle metadata | [crates/edgecrab-plugins/src/manifest.rs](../../../crates/edgecrab-plugins/src/manifest.rs#L347) `write_bundle_install_metadata` / `read_bundle_install_metadata` |
| Skill bodies | individual `SKILL.md` under `~/.edgecrab/skills/`, read on demand via the `skills` tool |

## Code is law: no runtime bundle concept

The word "bundle" in EdgeCrab refers to **install packaging** — a
downloaded archive of files (`manifest.rs#L347`). It answers "what was
installed together," not "load these N skills into my session now."

There is:

- No `/<name>` command that loads multiple skills at once.
- No bundle manifest that references member skills by name.
- No path that injects more than the per-skill *summary* — full skill
  bodies are pulled one at a time by the `skills` tool.

So the runtime workflow-priming feature is genuinely absent; only the
single-skill primitives exist.

## What to reuse

- `SkillsManifest::build` already enumerates skills — a bundle resolver
  can sit on top of it.
- The `/skills` subcommand dispatcher in `plugins.rs` is the natural home
  for `/skills bundle …`.
- `load_skill_summary` already knows how to read and format skill bodies
  — a bundle load reuses it per member.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
