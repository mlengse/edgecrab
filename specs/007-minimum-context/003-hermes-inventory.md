# 003 — Hermes Turn-1 Context Inventory (Code Is Law)

Source files:

- `hermes-agent/agent/system_prompt.py` — three-tier assembly
- `hermes-agent/agent/prompt_builder.py` — constants + context files + skills index
- `hermes-agent/model_tools.py` — schema provider
- `hermes-agent/toolsets.py` — `_HERMES_CORE_TOOLS`

---

## Assembly pipeline (first session turn)

```
  AIAgent.__init__ / conversation_loop
         │
         ▼
  _build_system_prompt()  →  build_system_prompt()
         │
         ├── build_system_prompt_parts()
         │        ├── stable   (identity, guidance, skills INDEX, coding blocks)
         │        ├── context  (system_message + AGENTS.md walk)
         │        └── volatile (memory, USER.md, date line)
         │
         └── join with "\n\n" → agent._cached_system_prompt

  get_tool_definitions(enabled_toolsets, disabled, quiet_mode=True)
         │
         └──► agent.tools (separate API field)
```

**Invariant (documented in `system_prompt.py:6–8`):** Prompt rebuilt only on compression / invalidation — **not** every turn. Same goal as EdgeCrab `cached_system_prompt`.

---

## Three-tier model (Hermes-specific)

```
  ┌─────────────────────────────────────────────────────────────┐
  │ STABLE TIER                                                 │
  │  SOUL.md OR DEFAULT_AGENT_IDENTITY                          │
  │  + HERMES_AGENT_HELP_GUIDANCE                               │
  │  + TASK_COMPLETION_GUIDANCE (all models, config-gated)      │
  │  + tool guidance blob (memory, session_search, skills, …)   │
  │  + STEER_CHANNEL_NOTE                                       │
  │  + computer_use block (if tool present)                     │
  │  + nous subscription block (if applicable)                │
  │  + tool-use enforcement + model-specific guidance           │
  │  + SKILLS INDEX (full build_skills_system_prompt)           │
  │  + coding_system_blocks() (if coding posture active)        │
  │  + env_probe one-liner (if non-default Python toolchain)    │
  │  + active profile hint                                      │
  │  + platform hint                                            │
  └─────────────────────────────────────────────────────────────┘
                              │
  ┌───────────────────────────▼─────────────────────────────────┐
  │ CONTEXT TIER                                                │
  │  optional caller system_message                             │
  │  + AGENTS.md / .cursorrules / CLAUDE.md / .hermes.md walk   │
  └─────────────────────────────────────────────────────────────┘
                              │
  ┌───────────────────────────▼─────────────────────────────────┐
  │ VOLATILE TIER                                               │
  │  MEMORY.md snapshot                                         │
  │  + USER.md profile                                          │
  │  + external memory provider block                           │
  │  + "Conversation started: <weekday, month day, year>"       │
  │    (+ session id, model, provider)                          │
  └─────────────────────────────────────────────────────────────┘
```

**WHY three tiers:** Comments in `system_prompt.py` separate build order. **At the API wire**, all three are joined into **one** system string — see [006-cache-preservation.md](006-cache-preservation.md).

---

## At the API wire (critical for cache)

```
  build_system_prompt_parts()     build_system_prompt()
        stable                          │
        context      ─── join ──────────┼──► ONE system message
        volatile                          │
                                        ▼
                          apply_anthropic_cache_control (system_and_3)
                          cache_control on last part of that ONE string
```

EdgeCrab emits **two** system messages when caching is on. Hermes emits **one**. Build-tier “stable” in Hermes ≠ a separate cache breakpoint.

---

## Stable tier — measured constants

| Block | ~Tokens | Always? |
|-------|---------|---------|
| `DEFAULT_AGENT_IDENTITY` | ~128 | if no SOUL.md |
| `HERMES_AGENT_HELP_GUIDANCE` | ~140 | **always** |
| `TASK_COMPLETION_GUIDANCE` | ~192 | if tools + `task_completion_guidance` (default on) |
| `MEMORY_GUIDANCE` + `SESSION_SEARCH` + `SKILLS_GUIDANCE` | ~500 combined | tool-gated, joined with spaces |
| `STEER_CHANNEL_NOTE` | ~170 | if any tools |
| `TOOL_USE_ENFORCEMENT` + model guidance | 0 / ~880 | non-Claude only |
| `coding_system_blocks()` | **0 / ~864** | coding posture in git repo (**MEASURED** in edgecrab cwd) |
| Profile hint | ~80–120 | always |
| Platform hint (CLI) | ~80 | platform-specific |

**Claude + tools, no coding posture: ~1,100–1,400 tokens stable** (before skills index).

### Hermes-only stable content (vs EdgeCrab)

| Block | Purpose |
|-------|---------|
| `HERMES_AGENT_HELP_GUIDANCE` | Points to docs + `hermes-agent` skill |
| `TASK_COMPLETION_GUIDANCE` | Universal anti-stub / anti-fabrication |
| `STEER_CHANNEL_NOTE` | Trust model for mid-turn `/steer` markers |
| `coding_system_blocks()` | Git branch, dirty state, operating brief |
| `env_probe` line | PEP 668 / uv / python path hints |
| Skills index in **stable build tier** | Larger cache hit when string unchanged; date/memory invalidate all |

---

## Prefix / KV cache (turn 2+)

**Strategy:** `system_and_3` — 4 breakpoints (`agent/prompt_caching.py`).

```
  BP1: entire system message (stable + context + volatile joined)
  BP2–4: last 3 non-system messages
```

**Provider policy:** `anthropic_prompt_cache_policy()` in `agent_runtime_helpers.py:1206` — native Anthropic, OpenRouter Claude, Nous Portal, Qwen/Alibaba, MiniMax Anthropic-wire, etc.

**TTL:** `prompt_caching.cache_ttl` in config — `"5m"` default, `"1h"` optional.

**Session restore:** Gateway reuses `_cached_system_prompt` verbatim from SQLite (`conversation_loop.py` `_restore_or_build_system_prompt`) so prefix bytes match across per-turn agent instances.

**Memory:** Frozen snapshot at load — mid-session writes do not mutate prompt (`tools/memory_tool.py:119`).

**Local KV:** `conversation_loop.py` strips content + canonicalizes tool-call JSON (`sort_keys`) for bit-perfect prefixes on Ollama/vLLM.

**vs EdgeCrab:** Hermes wins **route breadth** and **local prefix hygiene**; loses **stable-only survival** on date/memory/AGENTS churn because volatile bytes sit in the same system string as skills + guidance.

Details: [006-cache-preservation.md](006-cache-preservation.md).

---

## Context tier

Built when `not agent.skip_context_files` (`system_prompt.py:330–338`).

| Source | Discovery |
|--------|-----------|
| SOUL.md | Already consumed as identity if loaded |
| AGENTS.md | Walk cwd → git root |
| CLAUDE.md, .cursorrules, .hermes.md | Same walk |
| Injection scan | `scan_for_threats(scope="context")` — block with placeholder |

Truncation: `CONTEXT_FILE_MAX_CHARS = 20_000` — same as EdgeCrab.

**Not in cached prompt:** `ephemeral_system_prompt` — appended at API time only (`system_prompt.py:325–326`). EdgeCrab has similar separation for some gateway paths.

---

## Volatile tier

| Block | Notes |
|-------|-------|
| Memory | `_memory_store.format_for_system_prompt("memory")` — snapshot at load |
| USER.md | Always when user profile enabled |
| External memory | Optional `MemoryManager` additive block |
| Timestamp | **Date-only** (not minute) — PR #20451; reduces cache invalidation |

EdgeCrab adopted date-only policy in dynamic zone (`prompt_builder.rs:1304–1307`) — explicit Hermes port.

---

## Tool schemas

### `_HERMES_CORE_TOOLS`: 49 names (`toolsets.py:31–76`)

Includes kanban + computer_use **names** but many are **`check_fn` gated** off in normal CLI.

### Default policy

```python
enabled_toolsets=None  # → all toolsets, filtered by check_fn + disabled list
```

### MEASURED schema sizes (this machine, `get_tool_definitions`)

| Policy | Tools | Schema ~tokens |
|--------|-------|----------------|
| `None` (default) | **35** | **~15,096** |
| `["coding"]` | 28 | ~11,534 |
| `["file", "terminal"]` | **6** | **~3,186** |

**WHY fewer than 49:** Runtime gates hide kanban, computer_use, HA, send_message, browser, etc. when prerequisites missing. **Hermes default is smaller than the core list on paper.**

EdgeCrab applies similar `check_fn` gating but starts from a **larger registered set** (Honcho, extra process tools, web_crawl, …).

### Schema caching

`model_tools.py` caches definitions when `quiet_mode=True` — avoids ~7ms registry walk per call. EdgeCrab rebuilds from registry each turn (Rust — fast, but no cross-call cache).

---

## Skills index

`build_skills_system_prompt()` output lands in **stable** tier when skill tools present.

Hermes optimizations (see [022-cold-start-perf](../001-gap-analysis-v14/022-cold-start-perf/002-hermes-reference.md)):

- Disk snapshot `~/.hermes/skills/.cache.json`
- Async rescan post-launch
- `coding_compact_skill_categories()` — focus mode demotes descriptions to names-only

EdgeCrab: in-process manifest cache (`SKILLS_CACHE`, mtime+size) — skills body in **dynamic** zone.

---

## ASCII: Hermes default first turn (M1, Claude, CLI, empty memory)

```
  ~TOKENS (MEASURED / EST)
  0        5K       10K       15K       20K       25K
  ├─────────┼─────────┼─────────┼─────────┼─────────┤
  │▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓│ tool schemas (~15.1K MEASURED)
  │▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓│
  │▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓│
  │░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│ stable (~1.5–2.5K)
  │░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│ + skills index (~0.5–2K)
  │░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│ + coding blocks (~0–864)
  │▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒│ context files (0–5K+)
  │▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒│ volatile (~50+ memory)
  │ user message                                                         │
  └──────────────────────────────────────────────────────────────────────┘

  TOTAL M1 (no AGENTS):  ~17–20K tokens before user speaks
  TOTAL M2 (hermes repo): AGENTS.md is 69KB → truncates to 20K chars (~5K tok)
```

---

## Honest assessment (Hermes)

### Strengths for cache preservation

1. **Broad `cache_control` activation** — not limited to native Anthropic.
2. **Session DB prompt restore** — gateway prefix stability.
3. **Frozen memory snapshot** — no mid-session system mutation.
4. **Date-only volatile stamp** — same-calendar-day reuse possible.
5. **Local JSON normalization** — Ollama/vLLM KV reuse.

### Weaknesses / costs

1. **`check_fn` aggressively hides tools** — 49-name core list → 35 active schemas (minimum context).
2. **`enabled_toolsets=None` still default** — same “ship everything eligible” posture.
3. **`HERMES_AGENT_HELP_GUIDANCE` always on** — 140 tokens every session.
4. **Coding posture adds ~864 tokens** in git repos.
5. **Single system string** — date rollover invalidates cache for skills + guidance together.
6. **`KANBAN_GUIDANCE` ~1K tok** when kanban worker env set.

---

## Config knobs (minimum context)

| Knob | Effect |
|------|--------|
| `skip_context_files=True` on agent | −context tier |
| `_memory_enabled=False` | −memory block |
| `agent.task_completion_guidance: false` | −192 tokens stable |
| `agent.tool_use_enforcement: false` | −enforcement blocks |
| `agent.environment_probe: false` | −probe line |
| `enabled_toolsets=["file","terminal"]` | **~3.1K schema tokens MEASURED** |
| Coding focus mode | compacts skill categories |

**M0 clean-room (Hermes):** skip context + memory + minimal toolsets → **~4–6K tokens** + user (EST).

---

## Cross-refs

- EdgeCrab mirror: [002-edgecrab-inventory.md](002-edgecrab-inventory.md)
- Matrix: [004-comparison-matrix.md](004-comparison-matrix.md)
- Leverage: [005-leverage-plan.md](005-leverage-plan.md)
- Cache: [006-cache-preservation.md](006-cache-preservation.md)
