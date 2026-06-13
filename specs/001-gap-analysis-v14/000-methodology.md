# Methodology — First Principles Gap Analysis

> **Refresh status (this pass):** the analysis was re-verified against the
> current EdgeCrab codebase and Hermes **v0.15.0/v0.15.1**. Eight folders
> are now ✅ DONE, three ◐ PARTIAL, and three new Tier-D gaps (031–033)
> were added from v0.15. See [README.md §0](README.md#0-implementation-status-snapshot-code-is-law-this-refresh).
>
> **Status legend:** ✅ DONE (verified in code, usually `proof/`) ·
> ◐ PARTIAL (primitive exists, gap remains) · ○ OPEN (not started).

## 1. Scoring Rubric

Every feature is scored across three axes on a 1–5 scale:

| Axis | Question | 1 (low) | 5 (high) |
|------|----------|---------|----------|
| **Impact** | If we ship this, how many user-perceived problems disappear? | Niche edge case | Core UX win for every user |
| **Value-per-Effort** | Lines of code added ÷ user-facing change | Multi-crate rewrite | Single trait + tool entry |
| **Risk** | Could this break existing flows / cache invalidation / security? | Sandboxed, additive | Modifies hot paths or trust boundary |

Tier assignment:

```
TIER S : impact ≥ 4  AND  value-per-effort ≥ 4  AND  risk ≤ 3
TIER A : impact ≥ 4  AND  value-per-effort ≥ 3  AND  risk ≤ 4
TIER B : impact ≥ 3  AND  value-per-effort ≥ 4
TIER C : impact ≥ 4  AND  value-per-effort ≤ 3      (strategic but expensive)
SKIP   : impact ≤ 2
```

## 2. First Principles Filter

The three primitives (from [README.md](README.md#1-first-principles-framing)):

1. **Reliability of long-horizon execution** — measured by *tool-error rate per turn*,
   *goal drift per 10 turns*, and *successful task completion %*.
2. **Trust in side-effects** — measured by *file-rollback rate*, *prompt-injection
   surface in tool outputs*, and *time to detect a bad write*.
3. **Cost per useful turn** — measured by *USD/turn*, *p50 latency*, and
   *developer minutes to recover from an error*.

A feature is "high-value" iff it moves **≥ 2 of the 3** primitives by a
measurable margin and the change can be implemented behind a clean Rust
abstraction (trait, builder, or pure function).

## 3. DRY / SOLID Constraints on Implementation Plans

Every `004-implementation-plan.md` must satisfy:

- **Single Responsibility:** new functionality lands in its own module; no
  god-files (`agent.rs`, `conversation.rs`, `registry.rs` stay slim).
- **Open/Closed:** extend via traits or `inventory::submit!` registrations —
  never modify the `ToolHandler` trait, `LLMProvider` trait, or
  `PlatformAdapter` trait signatures without an ADR.
- **Liskov:** new provider/transport implementations must be drop-in
  substitutable behind existing traits.
- **Interface Segregation:** prefer many small traits
  (`GoalStore`, `FileMutationVerifier`, `SemanticLinter`) over one wide trait.
- **Dependency Inversion:** `edgecrab-core` may not depend on
  `edgecrab-cli` or `edgecrab-gateway` — wire new dependencies through
  builder methods, not direct imports.
- **DRY:** if Hermes does the same thing in 3 places (e.g. injection scanning
  in context files, memory writes, *and* tool errors), the Rust port has
  exactly one `injection_scan(text: &str)` function reused everywhere.
  *(This is no longer hypothetical: the refresh found **four** drifting
  threat-pattern sources in EdgeCrab — see
  [031-promptware-brainworm-defense/](031-promptware-brainworm-defense/).)*

## 4. "Code Is Law" Verification

All gap claims in this analysis are grounded in two real workspaces:

- Hermes (Python): `/Users/raphaelmansuy/Github/03-working/hermes-agent/`
- EdgeCrab (Rust): `/Users/raphaelmansuy/Github/03-working/edgecrab/`

Where a `002-hermes-reference.md` cites a file, that file exists in the Hermes
checkout. Where a `003-edgecrab-current-state.md` says something is missing,
a code search returned zero matches (or only stubs).

Line numbers are deliberately omitted — they drift weekly. Cited paths
remain stable. *(Exception: a few `003-edgecrab-current-state.md` docs in
the v0.15 refresh cite anchor lines for precise grounding; treat those as
indicative, not exact.)*

## 5. Brutal Honesty Clause

Each `001-overview.md` ends with a short **"What EdgeCrab gets wrong today"**
section. We do not sugarcoat. If a tool is half-implemented, untested, or
silently degrades, we say so. The point of this document is to *ship*, not
to make anyone feel good.

## 6. Out of Scope

- Refactors that do not unlock a new user-facing capability.
- Features that depend on Hermes-only Python ecosystems (e.g. PyTorch RL
  training rigs) without a clean Rust alternative.
- Cosmetic changes (colour theme tweaks, log format polish) — handled
  ad-hoc, not in this gap analysis.
- Features that exist solely to compete on benchmark numbers without
  improving developer outcomes.

## 7. Cross-References

- [README.md](README.md) — ranked index of all 30 features.
- [999-roadmap.md](999-roadmap.md) — sequenced execution plan.
- [../../AGENTS.md](../../AGENTS.md) — current EdgeCrab architecture
  reference (single source of truth).
