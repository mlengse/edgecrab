# 007 — Minimum Context at Start (EdgeCrab vs Hermes)

**Code is law.** This spec set measures what each harness *actually* sends on turn 1 — and how each harness preserves prefix/KV cache on turn 2+.

**Status:** Audited against `edgecrab` + `hermes-agent` repos, June 2026. Hermes schema sizes **MEASURED**; EdgeCrab schema sizes **EST** (CI budget test planned — see [005-leverage-plan.md](005-leverage-plan.md) L4.1).

---

## Two questions (do not merge them)

| Question | Doc |
|----------|-----|
| How many tokens on **turn 1**? | [001](001-first-principles.md) · [004](004-comparison-matrix.md) |
| How cheap is **turn 2+** (prefix / KV cache)? | [006-cache-preservation.md](006-cache-preservation.md) |

Minimum context and cache architecture overlap (stable vs dynamic split) but optimize for **different metrics**.

---

## Brutal verdict — minimum context (turn 1)

| Question | Answer |
|----------|--------|
| Who wins **M0 clean-room**? | **Hermes** — ~4–6K vs EC ~7–9K EST |
| Who wins **M1 default CLI**? | **Hermes, slightly** — ~15.1K tool schemas **MEASURED** vs EC ~17–22K EST; ~1.1–2.3K stable guidance vs EC **~2.3K measured** |
| Who wins **M2 realistic dev** (AGENTS.md cwd)? | **≈ tie** — both ~22–28K; tools + truncated AGENTS dominate |
| Biggest shared sin? | **`enabled_toolsets: None` = all eligible tools** on both codebases |

**Measured anchors (Hermes, `get_tool_definitions`, this workspace):**

| Policy | Tools | Schema ~tokens |
|--------|-------|----------------|
| `enabled_toolsets=None` (default) | 35 | **15,096** |
| `["file", "terminal"]` (minimal) | 6 | **3,186** |

EdgeCrab: `CORE_TOOLS` lists **64** names (`toolsets.rs`); comment still says “~45”. Default active count **EST ~45–55**; stable guidance with default core tools **~2,269 tok measured** (constant sum in `prompt_builder.rs`).

---

## Brutal verdict — cache preservation (turn 2+)

| Question | Answer |
|----------|--------|
| Who has **better API architecture**? | **EdgeCrab** — two system blocks; stable gets `cache_control`, dynamic does not |
| Who **activates** caching on more routes? | **Hermes** — OpenRouter, Nous, Qwen, MiniMax, native Anthropic |
| Who survives **date / memory / AGENTS** churn? | **EdgeCrab stable block** still hits; Hermes invalidates whole system string |
| Who caches **skills index** in cloud prefix? | **Hermes** (when full string byte-identical); EC pays skills in dynamic every turn |
| Local KV (Ollama / LM Studio)? | **Hermes** — JSON canonicalization + strip; EC uses prefill prune instead |

EdgeCrab prefix cache **shipped** per [004-prompt-prefix-cache/plan.md](../001-gap-analysis-v14/004-prompt-prefix-cache/plan.md) — `build_chat_messages_blocks`, `cache.prompt_prefix.ttl: "1h"`. Stale: `004-prompt-prefix-cache/003-edgecrab-current-state.md`.

---

## What “minimum context” means here

```
  TURN 1 INPUT TO MODEL
  ┌─────────────────────────────────────────────────────────────┐
  │ A. System prompt (stable + dynamic + context files + …)      │
  │ B. Tool schemas (`tools[]` — often LARGER than A)            │
  │ C. User message                                              │
  │ D. History (empty on cold start)                             │
  └─────────────────────────────────────────────────────────────┘

  Minimum context = min(A + B + C)     [M0 / M1 / M2 modes in 001]
  Cache cost      = f(A_stable, turn N) [006]
```

---

## Document map

| Doc | Purpose |
|-----|---------|
| [001-first-principles.md](001-first-principles.md) | Definitions, M0/M1/M2, leverage stack |
| [002-edgecrab-inventory.md](002-edgecrab-inventory.md) | EdgeCrab turn-1 + prefix cache law |
| [003-hermes-inventory.md](003-hermes-inventory.md) | Hermes turn-1 + prefix cache law |
| [004-comparison-matrix.md](004-comparison-matrix.md) | Side-by-side numbers (minimum + cache summary) |
| [005-leverage-plan.md](005-leverage-plan.md) | ROI-ordered fixes (P0–P4) |
| [006-cache-preservation.md](006-cache-preservation.md) | KV / Anthropic prefix cache deep dive |

---

## Cross-references

| Topic | Spec |
|-------|------|
| Stable/dynamic composition order | [effective_prompt/06-composition-order.md](../effective_prompt/06-composition-order.md) |
| EdgeCrab cache implementation plan | [004-prompt-prefix-cache/plan.md](../001-gap-analysis-v14/004-prompt-prefix-cache/plan.md) |
| Cold-start latency (not token count) | [022-cold-start-perf/](../001-gap-analysis-v14/022-cold-start-perf/) |
| Harness shape | [improve_plan/31-harness-deep-comparison.md](../improve_plan/31-harness-deep-comparison.md) |
| Hermes caching docs | `hermes-agent/website/docs/developer-guide/context-compression-and-caching.md` |

**Source of truth (code):**

- EdgeCrab: `crates/edgecrab-core/src/prompt_builder.rs`, `conversation.rs` (`build_chat_messages_blocks`)
- Hermes: `agent/system_prompt.py`, `agent/prompt_caching.py`, `agent/agent_runtime_helpers.py`

---

## Methodology

Token estimates: **chars ÷ 4** unless noted. Tags: **MEASURED** | **EST**. When code and docs disagree, **code wins**.
