# 033 — Skill Bundles (one command, many skills)

**Tier:** B | **Impact:** 3 | **Value-per-Effort:** 4 | **Risk:** 1
**Primitive moved:** Reliability of long-horizon execution (priming a workflow in one step)
**Source:** Hermes v0.15.0 — skill bundles invoked as `/<bundle-name>`

## Why It Matters (First Principles)

A "skill" is a unit of procedural knowledge. Real workflows need
**several** skills at once: a "release" workflow might need the
changelog-writing skill, the semver skill, and the git-tag skill
together. Loading them one at a time is friction that compounds over a
long session — the user (or agent) must remember and name each one.

A **bundle** is a named set of skills loaded with a single command. It
turns "remember these 4 skills and load each" into "`/release`." This is
a reliability lever: the agent starts the workflow already primed with
the complete, curated context, instead of discovering missing skills
mid-task.

## The Gap

EdgeCrab has a rich *per-skill* system — discovery, summary injection,
install, security scan — but no concept of a **named group** that loads
multiple skills with one invocation. The closest thing,
`read_bundle_install_metadata`, is **install-time packaging**, not a
runtime session-priming command (see
[003-edgecrab-current-state.md](003-edgecrab-current-state.md)).

## What "Good" Looks Like

- A bundle manifest (`bundles.yaml` or `skills/<name>/bundle.yaml`) names
  member skills.
- `/<bundle-name>` (or `/skills bundle load <name>`) injects every member
  skill body into the active context in one step.
- Bundles compose existing skills by reference — no skill content is
  duplicated (DRY).

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md) · [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
