# 033 — Hermes Reference

## Source

Hermes v0.15.0 — skill bundles: a single `/<bundle-name>` command loads a
curated set of skills into the session at once.

## What Hermes Did

| Aspect | Hermes behaviour |
|--------|------------------|
| Definition | A bundle is a named manifest listing member skills by reference |
| Invocation | `/<bundle-name>` loads every member skill in one command |
| Composition | Members are existing skills — the bundle holds references, not copies |
| Use case | Prime a multi-skill workflow (release, research, triage) in one step |

## Threat / Reliability Model

- **Reliability:** the agent begins a workflow already holding the full
  curated skill set, removing the "forgot to load skill X" failure mode
  mid-task.
- **UX:** one memorable command vs. N skill names.

## Design Principle

A bundle is a *thin index over existing skills*. It must not duplicate
skill bodies — it references them, so editing a member skill updates
every bundle that includes it.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
