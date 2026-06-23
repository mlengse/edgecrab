# 005 — Leverage Plan: Make EdgeCrab More Efficient (First Principles)

Ordered by **tokens saved per engineering hour**. Each item cites **WHY** (first principle) and **Hermes precedent** where applicable.

---

## Priority map

```
  P0 ─── default policy (fixes 80% of user pain, 20% of code)
   │
  P1 ─── schema diet (tools[] dominates)
   │
  P2 ─── guidance consolidation (EdgeCrab-specific debt)
   │
  P3 ─── lazy / deferred loading (cold start + first turn)
   │
  P4 ─── tier / cache polish (cost, not minimum)
```

---

## P0 — Change the default product shape

### L0.1 Default `enabled_toolsets` to `["core"]`, not `None`

| | |
|--|--|
| **WHY** | Principle 4: default config IS the product. `None` bypasses the carefully designed `core` alias that excludes LSP/MOA. |
| **Savings** | ~3–8K schema tokens vs current EST default |
| **Hermes** | Same sin (`None` = all). **Opportunity to leapfrog Hermes.** |
| **Law** | `config.rs` `ToolsConfig::default()` · `toolsets.rs:344–356` |
| **Risk** | Users expect browser/cron on first run — mitigate with doctor message + `edgecrab setup` explicit “full” profile |

**Implementation sketch:**

```yaml
# new default in DEFAULT_CONFIG
tools:
  enabled_toolsets: ["core"]   # was: null
```

Add `tools.profile: full | core | minimal` preset in setup wizard.

---

### L0.2 Ship a documented `minimal` profile for subagents AND user-facing `/tools profile minimal`

| | |
|--|--|
| **WHY** | Subagents already set `skip_context_files + skip_memory` — but still inherit full tool schemas unless toolsets narrowed. |
| **Savings** | M0 ~7–9K → ~4–6K (match Hermes floor) |
| **Hermes** | `delegate_task` inherits parent toolsets with narrowing logic |
| **Law** | `sub_agent_runner.rs:102–104` · `toolsets.rs:362` `minimal` alias |

---

## P1 — Schema diet (biggest physics)

### L1.1 Reconcile `CORE_TOOLS` to ≤45 names or update budget comments

| | |
|--|--|
| **WHY** | Principle 1: schemas dominate. 64 names in `CORE_TOOLS` with a “~45 / ~18K tok” comment means nobody owns the budget. |
| **Action** | Audit each tool: **schema-required vs discoverable via `mcp_list_tools` / skills** |
| **Candidates to demote out of default core** | Honcho×6 (opt-in `honcho` toolset), extra process tools (collapse to `process` meta), `skills_hub`, `pdf_to_markdown`, `web_crawl` → `research` toolset |
| **Hermes** | Keeps leaner `_HERMES_CORE_TOOLS`; defers video/x_search to opt-in toolsets |

---

### L1.2 Lazy schema materialization (Hermes parity)

| | |
|--|--|
| **WHY** | Turn 1 pays for every description byte even if model never calls the tool. |
| **Hermes** | `model_tools.py` caches computed definitions; tools register lazily on first dispatch in some paths ([022-cold-start-perf](../001-gap-analysis-v14/022-cold-start-perf/002-hermes-reference.md)) |
| **EdgeCrab action** | Two-phase schema: **core 12 tools full schema + `tool_search` / compact index for long tail** OR tiered `tools_mode: compact|full` config |
| **Savings** | Potentially 40–60% schema tokens in compact mode |

---

### L1.3 ACP must not load LSP+MOA unless `enabled_toolsets` requests it

| | |
|--|--|
| **WHY** | Editor integration currently pays ~7.6K LSP schema + ~378 tok guidance by static `ACP_TOOLS` list. |
| **Savings** | ~8K tokens for VS Code users who never got asked |
| **Law** | `toolsets.rs:202–279` `ACP_TOOLS` |
| **Fix** | Derive ACP from `acp_tools()` runtime + default `enabled_toolsets: ["core", "lsp"]` only when LSP server detected |

---

## P2 — Guidance consolidation (EdgeCrab debt)

### L2.1 Add `TASK_COMPLETION_GUIDANCE` equivalent (port from Hermes)

| | |
|--|--|
| **WHY** | One ~192-token universal law beats five feature-specific “don’t stop early” blocks scattered across scheduling/file/research guidance. |
| **Hermes** | `TASK_COMPLETION_GUIDANCE` in stable tier, all models (`system_prompt.py:105–112`) |
| **Action** | Add `TASK_COMPLETION_GUIDANCE` const; inject when any tools loaded; **then trim** redundant sentences from `FILE_OUTPUT_ENFORCEMENT`, `RESEARCH_TASK`, `PROGRESSION` |

**Net savings target:** ~400–700 stable tokens without behavior loss.

---

### L2.2 Collapse scheduling + messaging + delivery into schema + one-line pointers

| | |
|--|--|
| **WHY** | `SCHEDULING_GUIDANCE` (~536 tok) duplicates what `manage_cron_jobs` JSON schema + tool description should enforce (Principle 3: gating beats prose). |
| **Action** | Move action→intent tables into tool description examples; keep ≤3-line system reminder |
| **Hermes** | No scheduling prose block — relies on tool docs |
| **Savings** | ~700–900 stable tokens on default core |

---

### L2.3 Merge vision disambiguation into tool descriptions only

| | |
|--|--|
| **WHY** | `VISION_GUIDANCE` (~362 tok) exists because tool order biased models — fix order + schema `description` first; drop prompt block if eval passes |
| **Savings** | ~362 stable tokens |

---

### L2.4 LSP guidance only when `enabled_toolsets` contains `lsp`

| | |
|--|--|
| **WHY** | Already gated on tool presence — ensure ACP default doesn’t load LSP tools silently (see L1.3) |
| **Savings** | ~378 tokens when LSP absent |

---

## P3 — Deferred / lazy context (turn-1 floor)

### L3.1 Skills index: compact stable + full dynamic (cache-aware)

| | |
|--|--|
| **WHY** | Skills index is 0.5–2K tokens even for trivial prompts. |
| **Cache trade-off** | Skills in stable ↑ cloud cache mass; skills in dynamic ↑ minimum turn-1 cost but preserves EC stable BP on skill churn |
| **Recommendation** | **Hybrid ([006](006-cache-preservation.md)):** name-only index in stable (~100 tok); full bodies on `skill_view` or in dynamic |
| **Hermes** | Full index in stable build tier → cached as part of one system string when bytes match |

---

### L3.2 Context file cache with mtime key (Hermes parity)

| | |
|--|--|
| **WHY** | Cold start + session resume re-reads AGENTS.md; doesn’t add tokens but delays first turn. For minimum **time**, not tokens. |
| **Hermes** | Context file cache in [022 reference](../001-gap-analysis-v14/022-cold-start-perf/002-hermes-reference.md) |
| **EdgeCrab** | Partial — injection scan every build; add `(path, mtime, size) → content` cache |

---

### L3.3 Optional `skip_context_files` default for cron/gateway headless profiles

| | |
|--|--|
| **WHY** | Cron has no project context need — Hermes cron platform hint says autonomous mode |
| **Law** | `Platform::Cron` already has `CRON_HINT`; add config profile auto-skipping AGENTS walk |

---

## P4 — Cache polish + measurement (prevent regression)

### L4.1 CI token budget test

```
  Assert: default CLI profile (core toolset, Claude, skip memory)
          system_prompt + tools JSON < 18_000 tokens

  Assert: minimal profile < 8_000 tokens
```

**WHY:** Code is law — if CI doesn’t measure it, `CORE_TOOLS` will grow to 80 again.

---

### L4.2 `/context budget` slash command (doctor for tokens)

Print breakdown:

```
  stable:    2,847 tok
  dynamic:   1,203 tok
  tools:    14,992 tok (35 tools)
  ─────────────────────
  total:    19,042 tok (14.9% of 128K)
```

Hermes TUI exposes system prompt + tools in debug paths (`tui_gateway/server.py`); EdgeCrab should match in `/doctor` or `/cost`.

---

### L4.3 Port Hermes `anthropic_prompt_cache_policy()` breadth

| | |
|--|--|
| **WHY** | Two-block split is useless on OpenRouter/Qwen if `prompt_cache_config_for()` returns `None` |
| **Hermes** | `agent_runtime_helpers.py:1206–1308` |
| **Action** | Extend `provider_supports_prompt_caching()` + wire `build_chat_messages_blocks` for OpenRouter Claude, Nous, Qwen |
| **Savings** | Same stable BP architecture on routes users actually use |

---

### L4.4 Local KV normalization (Ollama / LM Studio)

| | |
|--|--|
| **WHY** | Local servers reuse KV on **byte-identical** prefixes; tool JSON key order breaks hits |
| **Hermes** | `conversation_loop.py` ~712–743 — strip + `sort_keys` on tool args |
| **Action** | Canonicalize tool-call JSON in `append_conversation_messages` when provider is local |
| **Complements** | Existing prefill prune (014) — prune shrinks size; normalize preserves hits |

---

### L4.5 Optional semi-stable skills breakpoint (advanced)

| | |
|--|--|
| **WHY** | Fits Anthropic 4-BP budget: stable (1h) + skills (5m) + 2 rolling message BPs |
| **Spec** | [006-cache-preservation.md](006-cache-preservation.md) target diagram |
| **Risk** | Uses 2 of 4 breakpoints on system alone — validate against `apply_cache_control` message BPs |

---

## ASCII: target state after P0+P1+P2

```
  TODAY (EC default M1)              TARGET (core profile + guidance trim)
  ─────────────────────              ─────────────────────────────────────

  tools ████████████████████ ~18K    tools ███████████████ ~12K
  guide ███ ~2.3K                  guide ██ ~1.5K (post L2 trim)
  ctx   ██ ~0–5K                     ctx   ██ ~0–5K
  ─────────────────                  ─────────────────
  ~21–24K                            ~14–17K   (−30% floor)
```

Still not “minimal” — **that requires `minimal` toolset (~8K total)** — but default becomes honest.

---

## What NOT to do

| Anti-pattern | Why |
|--------------|-----|
| Shrink `DEFAULT_IDENTITY` only | Saves ~20 tokens; theater |
| Disable injection scanning | Security regression for ~0 token win |
| Put datetime back in stable zone | Breaks prefix cache (see [effective_prompt/06](../effective_prompt/06-composition-order.md)) |
| Remove tool gating, keep prose | Doubles waste |

---

## Suggested execution order (sprints)

| Sprint | Items | Expected Δ | Status |
|--------|-------|------------|--------|
| S1 | L0.1, L4.1 | −3–8K default | **Done** — `config.rs`, `context_budget.rs` CI |
| S2 | L2.1, L2.2, L2.3 | −1–1.5K stable | **Done** — `TASK_COMPLETION_GUIDANCE` + trim |
| S3 | L1.1, L1.3 | −2–8K schema | **Done** — honcho/research demotion + core alias fix |
| S4 | L1.2, L3.1 | structural | Open |
| S5 | L4.2, L3.2 | observability | **Partial** — `/context budget` + doctor toolset warn done; mtime cache open |
| S6 | L4.3, L4.4, L4.5 | cache hits on more routes | **Partial** — L4.3/4.4 + base_url done; L4.5 open |

Post-implementation assessment: [007-implementation-assessment.md](007-implementation-assessment.md)

---

## Cross-refs

- [README.md](README.md) — verdict
- [004-comparison-matrix.md](004-comparison-matrix.md) — numbers
- [006-cache-preservation.md](006-cache-preservation.md) — cache law
- [specs/001-gap-analysis-v14/022-cold-start-perf/](../001-gap-analysis-v14/022-cold-start-perf/) — latency
- [specs/001-gap-analysis-v14/004-prompt-prefix-cache/plan.md](../001-gap-analysis-v14/004-prompt-prefix-cache/plan.md) — shipped EC cache
