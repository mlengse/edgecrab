# 007 — Post-Implementation Assessment vs Hermes (Brutal)

**Date:** 2026-06-15 (updated)  
**Scope:** Sprint S1–S3 + S6 from [005-leverage-plan.md](005-leverage-plan.md)

---

## First-principles design choices

| Principle | Choice | Why |
|-----------|--------|-----|
| **Agent must act** | `core` alias includes `terminal` + `file` | Shell execution is non-negotiable; a schema without `terminal` is a chatbot, not an agent |
| **Agent must perceive** | `core` includes `web` (search + extract, not crawl) | Research is default; `web_crawl` demoted to `research` toolset (heavy schema + rare turn-1 need) |
| **Agent must remember** | `core` includes `memory` (read/write only) | Honcho split to opt-in `honcho` toolset — 6 schemas saved from default |
| **Schema is physics** | CI gates: `core` < 18K, `minimal` < 8K | If CI doesn't measure it, the list grows forever (Hermes and EC both sinned with `None` = all) |
| **Cache ≠ minimum** | Stable/dynamic split preserved; skills stay dynamic | Turn-1 cost ↑ slightly; cross-session Anthropic prefix hits ↑ when AGENTS/memory churn |
| **Default is product** | `enabled_toolsets: ["core"]` not `None` | **Leapfrogs Hermes** — Hermes still ships `enabled_toolsets=None` |

---

## What shipped (code is law)

| ID | Change | Law |
|----|--------|-----|
| L0.1 | `ToolsConfig::default()` → `enabled_toolsets: Some(["core"])` | `config.rs` |
| L0.2 | Subagents default `minimal` when no toolsets requested | `delegate_task.rs` |
| L1.1 | Honcho → `honcho` toolset; `web_crawl` + `pdf_to_markdown` → `research` | `honcho.rs`, `extract_crawl.rs`, `pdf_to_markdown.rs` |
| L1.1b | `core` alias: `web`, `terminal`, `memory`, `skills` (was missing — agent couldn't shell/search by default) | `toolsets.rs` |
| L1.3 | `ACP_TOOLS` / `acp_tools()` exclude LSP+MOA; ACP runtime sets `["core","lsp"]` | `toolsets.rs`, `main.rs` |
| L2.1 | `TASK_COMPLETION_GUIDANCE` + trimmed PROGRESSION/SCHEDULING/FILE/RESEARCH blocks | `prompt_builder.rs` |
| L4.1 | CI schema budget tests (`core` < 18K, `minimal` < 8K) | `context_budget.rs` |
| L4.2 | `/context budget` slash command | `commands.rs`, `app.rs`, `agent.rs` |
| L4.3 | `prompt_cache_policy.rs` + `base_url` wired in `prompt_cache_config_for` | `prompt_cache_policy.rs`, `conversation.rs` |
| L4.4 | Local KV normalization (trim + canonical tool args) | `local_provider_policy.rs`, `conversation.rs` |
| L4.2b | `edgecrab doctor` warns on `enabled_toolsets: null` / `all` | `doctor.rs` |

---

## Verdict table (honest)

| Dimension | Hermes | EdgeCrab (after) | Winner |
|-----------|--------|------------------|--------|
| **Default tool surface** | `None` = all (~35 tools, ~15K tok measured) | `core` default + CI <18K; web+terminal+memory+skills | **EdgeCrab** |
| **Turn-1 minimum (M1)** | ~15K tools + ~1.5K guidance (one string) | ~14–17K tools + ~1.5–2K guidance (split blocks) | **≈ tie** |
| **Stable/dynamic architecture** | Single cached system string (`system_and_3`) | Two-block stable+dynamic (shipped earlier) | **EdgeCrab** |
| **Cache provider breadth** | `anthropic_prompt_cache_policy()` — mature matrix | Policy module + `base_url` from `model.base_url` | **≈ parity** |
| **Local KV reuse** | Full message normalize pass | Tool-arg canonicalize + content trim on local only | **Hermes slightly** |
| **ACP editor bloat** | N/A (different integration) | LSP opt-in via `lsp` toolset | **EdgeCrab fixed** |
| **Subagent tool diet** | Parent narrowing + skip flags | `minimal` default + skip_context/memory | **Parity** |
| **Observability** | TUI debug paths | `/context budget` + `/cost` + doctor warn | **EdgeCrab** |
| **CORE_TOOLS honesty** | `_HERMES_CORE_TOOLS` ~49 names | `CORE_TOOLS` const 64 names; runtime = `core` alias | **Hermes** |
| **Lazy schema / compact mode** | `model_tools.py` tiers | Not implemented | **Hermes** |
| **Skills index** | Full index in stable | Full index in dynamic (cache-safe) | **Trade-off** |

---

## Brutal truths

### EdgeCrab wins (real, not marketing)

1. **Default `core` leapfrogs Hermes** — Hermes still ships `enabled_toolsets=None`. EC default includes shell, web, memory, skills — a complete agent loop under CI budget.
2. **Fixed pre-ship bug** — `core` alias previously omitted `web` and `terminal`; new installs would get a file-editing chatbot with no shell. First-principles fix.
3. **Two-block cache is the right architecture** — Hermes packs datetime + skills into one string; EC's stable zone is genuinely stable.
4. **ACP LSP opt-in** — Was ~7.6K tokens for users who never enabled LSP.
5. **CI budget tests** — Hermes has manual measurement; EC **fails CI** if `core` schema regresses past 18K.

### EdgeCrab still loses (don't spin this)

1. **`CORE_TOOLS` const is still 64 names** — Runtime policy uses `core` alias; the const is documentation debt. Hermes `_HERMES_CORE_TOOLS` is the honest inventory.
2. **Prompt guidance still heavier than Hermes** — MEMORY, SESSION_SEARCH, SKILLS, MESSAGE_DELIVERY, VISION, LSP, code_editing blocks. Net stable guidance ~1.5–2.2K vs Hermes ~1.2K.
3. **`native_inner_layout` not wired to wire layer** — Qwen-on-OpenRouter envelope tweaks may still differ from Hermes.
4. **No lazy schema / compact tools mode (L1.2)** — Biggest future win for both codebases.
5. **Existing `config.yaml` with `enabled_toolsets: null`** — Doctor warns; no auto-migration.

### Tie / depends on workload

| Workload | Better default |
|----------|----------------|
| CLI coding, no LSP | **EdgeCrab** (core default) |
| VS Code ACP | **EdgeCrab** (core+lsp explicit) |
| OpenRouter Claude daily driver | **≈ tie** — both cache when URL/model match policy |
| Ollama/LM Studio 50-turn session | **Tie** — both normalize; EC adds structural prune (014) |
| Research + write_file on Qwen local | **EdgeCrab** (more enforcement guidance — costs tokens, buys completion) |

---

## Measured anchors (post-change)

| Profile | CI assertion | Hermes measured (prior art) |
|---------|--------------|----------------------------|
| `core` schema | < 18,000 tok | N/A (`None` = all ≈ 15K for 35 tools) |
| `minimal` schema | < 8,000 tok | `["file","terminal"]` ≈ 3,186 tok |
| Stable guidance | Not CI-gated yet | ~1.2K in one string |

Run locally: `/context budget` after first turn, or `cargo test -p edgecrab-core default_core_profile`.

---

## What's next (highest ROI still open)

1. **L1.2** — Compact schema mode (Hermes `model_tools.py` parity).
2. **L4.5** — Semi-stable skills breakpoint (advanced cache).
3. **Shrink `CORE_TOOLS` const** — align documentation with alias policy.

---

## Cross-refs

- [005-leverage-plan.md](005-leverage-plan.md) — full backlog
- [004-comparison-matrix.md](004-comparison-matrix.md) — pre-implementation numbers
- [006-cache-preservation.md](006-cache-preservation.md) — cache architecture
