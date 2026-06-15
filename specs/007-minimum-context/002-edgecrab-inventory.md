# 002 — EdgeCrab Turn-1 Context Inventory (Code Is Law)

Source files:

- `crates/edgecrab-core/src/prompt_builder.rs` — system prompt assembly
- `crates/edgecrab-core/src/conversation.rs` — when prompt is built/cached
- `crates/edgecrab-tools/src/toolsets.rs` — tool policy
- `crates/edgecrab-tools/src/registry.rs` — schema emission

---

## Assembly pipeline (first session turn)

```
  conversation.rs::execute_loop (first provider.chat)
         │
         ▼
  session.cached_system_prompt is None?
         │
         yes ──► PromptBuilder::build_blocks()
         │              │
         │              ├── STABLE zone (behavioral law, tool-gated)
         │              └── DYNAMIC zone (timestamp, cwd, files, memory, skills)
         │
         └──► flatten → cached_system_prompt (+ cached_stable_prompt for cache API)
         
  registry.get_definitions(enabled_toolsets, disabled, ctx)
         │
         └──► tools[] attached to API call (SEPARATE from system string)
```

**WHY two channels:** OpenAI-compatible APIs carry tools outside the system string. Token accounting merges them on the provider side. **Minimum context = system + tools + user.**

Cache build site: `conversation.rs` ~845–968 (`build_blocks`, memory load, skill bundle).

---

## System prompt — stable zone

Built in `PromptBuilder::build_blocks()` (`prompt_builder.rs:1166+`).

| # | Block | Gated? | ~Tokens | Notes |
|---|-------|--------|---------|-------|
| 1 | `DEFAULT_IDENTITY` or SOUL override | always | ~138 | Shorter than Hermes identity |
| 2 | Platform hint (`CLI`, `Telegram`, …) | platform | ~40–120 | `platform_hint()` |
| 3 | `TOOL_USE_ENFORCEMENT_GUIDANCE` | non-Claude models | ~209 | Skipped for `claude`/`anthropic` |
| 4 | Model-specific guidance (GPT/Gemini/generic) | non-Claude | ~166–403 | `model_specific_guidance()` |
| 5 | `MEMORY_GUIDANCE` | `memory_write` | ~190 | |
| 6 | `SESSION_SEARCH_GUIDANCE` | `session_search` | ~47 | |
| 7 | `TASK_STATUS_GUIDANCE` | `report_task_status` | ~147 | **EC-only vs Hermes default** |
| 8 | `PROGRESSION_GUIDANCE` | `report_task_status` | ~144 | **EC-only** |
| 9 | `SKILLS_GUIDANCE` | `skill_manage` | ~97 | |
| 10 | `SCHEDULING_GUIDANCE` | `manage_cron_jobs` + not Cron platform | **~536** | **Largest EC-specific block** |
| 11 | `MESSAGE_DELIVERY_GUIDANCE` | `send_message` | ~221 | Hermes has no equivalent prose block |
| 12 | `MOA_GUIDANCE` | `moa` tool loaded | ~111 | Opt-in toolset — good |
| 13 | `VISION_GUIDANCE` | `vision_analyze` | **~362** | Disambiguate vs `browser_vision` |
| 14 | `COMPUTER_USE_GUIDANCE_COMPACT` | `computer_use` | ~? | Compact vs Hermes full block |
| 15 | `LSP_GUIDANCE` | any LSP tool in schema | **~378** | **ACP mode always hits this** |
| 16 | `code_editing_guidance_for_model()` | `apply_patch` or `write_file` | ~200–400 | Model-specific, owned string |
| 17 | `FILE_OUTPUT_ENFORCEMENT_GUIDANCE` | `write_file` | ~185 | FP34 — all model families |
| 18 | `RESEARCH_TASK_GUIDANCE` | write + web/search tools | ~198 | FP36 |

**Claude + default core tools — stable guidance sum: ~2,269 tok MEASURED**  
(sum of gated constants in `prompt_builder.rs`; excludes platform hint + `code_editing_guidance_for_model()`)

(vs Hermes ~1,100–1,400 tok stable prose before skills index — see [004-comparison-matrix.md](004-comparison-matrix.md))

### What EdgeCrab does NOT inject (Hermes does)

| Hermes block | EdgeCrab status |
|--------------|-----------------|
| `HERMES_AGENT_HELP_GUIDANCE` | ❌ no equivalent |
| `TASK_COMPLETION_GUIDANCE` (all models) | ❌ **gap** — no universal “finish the job / no fabrication” block |
| `STEER_CHANNEL_NOTE` | ❌ steering injected differently (message channel, not stable prompt) |
| `coding_system_blocks()` | ❌ no git/workspace operating brief in stable tier |
| Active profile hint | ❌ (single `~/.edgecrab/` home — simpler, smaller) |

---

## System prompt — dynamic zone

| # | Block | ~Tokens | Volatility |
|---|-------|---------|------------|
| D1 | Date + session ID + model | ~40–80 | Day-stable (matches Hermes PR #20451 policy) |
| D1b | Local inference geometry guidance | 0–200 | Only local models + mutation tools |
| D2 | Execution environment guidance | ~0–300 | cwd, allowed roots |
| D3 | Context files (`AGENTS.md`, `.edgecrab.md`, …) | **0–5K each file** | Per-repo; truncated at 20K chars |
| D4 | Memory sections (`MEMORY.md`, `USER.md`) | 0–∞ user | `skip_memory` to disable |
| D5 | Skills prompt (XML-wrapped index) | ~200–2K+ | Per `~/.edgecrab/skills/` |

**WHY skills in dynamic (not stable):** Skills change on `/skills install`. Keeping them out of `cached_stable_prompt` means the **Anthropic stable breakpoint** (~2.3K guidance) survives skill updates without a full stable rewrite. Trade-off: skills bytes are **never cloud-cache-read** (paid every turn). See [006-cache-preservation.md](006-cache-preservation.md).

**Hermes:** Skills index is assembled in the **stable build tier**, then **joined** with context + volatile into one API system string — can cache skills with guidance when the whole string is identical.

Context discovery: `discover_context_files()` walks cwd → git root; scans injection before inject (`prompt_builder.rs:1348–1397`).

Opt-out flags (`config.rs` / `AgentConfig`):

- `skip_context_files` — drops D3 (+ SOUL walk in builder path)
- `skip_memory` — drops memory load in `conversation.rs`

Subagents set both true (`sub_agent_runner.rs:102–104`) — **Hermes parity pattern**.

---

## Tool schemas — the elephant

### Default policy

```yaml
# config default (ToolsConfig)
enabled_toolsets: null   # means ALL toolsets eligible
disabled_toolsets: null
```

`registry.get_definitions(None, …)` includes every tool whose:

1. `is_available()` passes
2. `check_fn(ctx)` passes (runtime gates: gateway, HA token, browser, …)

### CORE_TOOLS list size

**64 tools** named in `CORE_TOOLS` (`toolsets.rs:24–104`) — comment says “~45” but **code lists 64**. Documentation drift is itself a smell.

Notable EC-only surface vs Hermes core:

- `web_crawl`, `pdf_to_markdown`, `transcribe_audio`, `generate_image`
- 7 extra process tools (`run_process`, `kill_process`, …)
- 3 extra browser tools (`browser_wait_for`, `browser_select`, `browser_hover`)
- Honcho 6-pack (`honcho_*`)
- `skills_hub`, `skills_categories`, `checkpoint`, `mcp_*`, split memory read/write

### `core` alias (setup wizard)

Expands to toolsets: `file`, `meta`, `scheduling`, `delegation`, `code_execution`, `session`, `mcp`, `messaging`, `media`, `browser` — **explicitly excludes LSP and MOA** (`toolsets.rs:344–356`).

**WHY:** Comment targets “schema payload under ~18K tokens”. Good intent; **`enabled_toolsets: None` bypasses this entirely.**

### ACP / editor mode

`ACP_TOOLS` includes **full LSP suite + MOA** (`toolsets.rs:202–279`) — editor sessions pay **~7.6K+ extra LSP schema tokens** by design.

### Schema size (EST)

| Mode | Tools active | Schema tokens |
|------|--------------|---------------|
| M0 `minimal` (file+terminal toolsets) | ~10–14 | **~4–5K EST** |
| M1 default (`enabled_toolsets: None`) | ~45–55 EST | **~17–22K EST** |
| ACP default | ~70+ with LSP | **~24–30K EST** |

Hermes M1 measured: **35 tools, ~15.1K** — EdgeCrab default is **heavier**.

---

## Prefix / KV cache (turn 2+) — shipped

EdgeCrab does **not** only split prompts for documentation — it emits **two system messages** at the API when caching is active.

```
  build_api_chat_messages()  conversation.rs ~2978
         │
         ├─ cached_stable_prompt + cache_cfg?
         │       yes → build_chat_messages_blocks()
         │                 sys[0] stable  + cache_control (ttl 1h default)
         │                 sys[1] dynamic (no cache_control)
         │       no  → build_chat_messages(combined)  ← Hermes-like fallback
         │
         └─ apply_cache_control on last N user messages (BP 2–4)
```

**Dual gate (both required for two-block path):**

| Config key | Default | Role |
|------------|---------|------|
| `model_config.prompt_caching` | `true` | Master switch |
| `cache.prompt_prefix.enabled` | `true` | Stable-block TTL tier |
| `cache.prompt_prefix.ttl` | `"1h"` | Cross-session stable reuse |
| `provider.name()` | `"anthropic"` only | **Gap vs Hermes routes** |

Law: `config.rs:106–140`, `conversation.rs:3051–3112`, `prompt_cache_config_for()` ~3166.

**Cache-safe patterns (EdgeCrab):**

- Goals → ephemeral **user** message, not system (`render_goal_block`, test at `conversation.rs:7523`)
- Memory → dynamic zone; snapshot at session build
- `/compress` → clears `cached_system_prompt` + `cached_stable_prompt` (`agent.rs`)
- Shadow judge → no new cache markers (`shadow_judge.rs:13–17`)

Full matrix: [006-cache-preservation.md](006-cache-preservation.md).

---

## ASCII: EdgeCrab default first turn (M1, Claude, CLI, empty memory)

```
  ~TOKENS (EST)
  0        5K       10K       15K       20K       25K       30K
  ├─────────┼─────────┼─────────┼─────────┼─────────┼─────────┤
  │▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓│ tool schemas (~18K)
  │▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓│
  │▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓│
  │░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│ stable guidance (~3K)
  │▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒│ context files (0–5K+)
  │▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒│ skills index (dynamic)
  │▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒│ timestamp (~50)
  │ user message                                                         │
  └──────────────────────────────────────────────────────────────────────┘

  TOTAL M1 (no AGENTS):  ~21–24K tokens before user speaks
  TOTAL M2 (this repo):  + up to ~9K if AGENTS.md injected (36KB file → truncated to 20K chars ≈ 5K tok)
```

---

## Honest self-assessment

### What EdgeCrab gets right

1. **Tool-gated guidance** — no `SCHEDULING_GUIDANCE` without cron tool (`has_tool` pattern).
2. **LSP/MOA out of `core` alias** — correct separation for schema budget.
3. **Stable/dynamic split** — datetime in dynamic zone (post composition-order fix).
4. **Subagent lean mode** — skips context + memory.
5. **Skills manifest cache** — mtime+size invalidation (Hermes parity).
6. **Two-block Anthropic cache** — stable guidance cached across date/memory/AGENTS churn.

### What EdgeCrab gets wrong (minimum-context lens)

1. **`enabled_toolsets: None` ships the universe** — the `core` alias discipline never applies by default.
2. **Guidance sprawl** — scheduling, messaging, vision, file-output, research, task-status blocks stack on default core.
3. **Missing `TASK_COMPLETION_GUIDANCE` equivalent** — Hermes ships universal finish/no-fabrication law; EC relies on scattered blocks.
4. **`CORE_TOOLS` comment lies (45 vs 64)** — team lost track of schema budget.
5. **ACP defaults heavier than CLI** — LSP guidance + schemas always on in editor path.
6. **Skills in dynamic** — correct for stable cache isolation; turn-1 still pays full skills index.
7. **Prefix cache provider scope** — native Anthropic only; OpenRouter/Qwen users get single-block fallback.

---

## Config knobs (minimum + cache)

| Knob | Effect |
|------|--------|
| `skip_context_files: true` | −context files tier |
| `skip_memory: true` | −MEMORY/USER |
| `tools.enabled_toolsets: ["minimal"]` | file + terminal toolsets only |
| `tools.enabled_toolsets: ["core"]` | alias expansion without LSP/MOA |
| `tools.disabled_toolsets: ["browser", "media", …]` | surgical cuts |
| `EDGECRAB_SKIP_CONTEXT_FILES=1` | env override |
| `EDGECRAB_SKIP_MEMORY=1` | env override |

| `cache.prompt_prefix.enabled: false` | Single system block; no stable BP |
| `cache.prompt_prefix.ttl: "5m"` | Shorter cross-session stable tier |

**M0 clean-room recipe:**

```yaml
skip_context_files: true
skip_memory: true
tools:
  enabled_toolsets: ["minimal"]
```

Expected floor: **~7–9K tokens** + user message (EST).

---

## Cross-refs

- Hermes mirror: [003-hermes-inventory.md](003-hermes-inventory.md)
- Numbers table: [004-comparison-matrix.md](004-comparison-matrix.md)
- Fixes: [005-leverage-plan.md](005-leverage-plan.md)
- Cache: [006-cache-preservation.md](006-cache-preservation.md)
