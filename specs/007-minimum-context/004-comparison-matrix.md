# 004 — Side-by-Side Comparison Matrix

**Legend:** MEASURED = computed in this audit · EST = structural estimate · ✅ better for minimum context · ❌ worse · ≈ parity

---

## Executive scorecard — minimum context (turn 1)

| Dimension | Hermes | EdgeCrab | Winner |
|-----------|--------|----------|--------|
| Default tool schema | ~15.1K tok **MEASURED** | ~17–22K tok **EST** | ✅ Hermes |
| Default stable guidance (Claude, core tools) | ~1.1–1.4K + skills | **~2.3K MEASURED** + skills in dynamic | ✅ Hermes |
| M0 clean-room | ~4–6K | ~7–9K EST | ✅ Hermes |
| M2 realistic dev | ~22–26K | ~24–28K EST | ≈ tie |
| Default toolset discipline | ❌ None=all | ❌ None=all | ❌ both |
| Universal task-completion law | ✅ | ❌ | ✅ Hermes |
| Scheduling guidance | 0 | ~536 tok | ✅ Hermes |

## Executive scorecard — cache preservation (turn 2+)

| Dimension | Hermes | EdgeCrab | Winner |
|-----------|--------|----------|--------|
| API stable/dynamic split | ❌ one system string | ✅ two system blocks | ✅ EC |
| Stable BP survives date/memory/AGENTS | ❌ | ✅ | ✅ EC |
| Skills in cloud cache prefix | ✅ if string identical | ❌ (dynamic tier) | Hermes when static |
| `cache_control` provider routes | Many | **Anthropic only** | ✅ Hermes |
| Conversation rolling cache (system_and_3) | ✅ | ✅ | ≈ tie |
| Local KV hygiene | ✅ JSON sort + strip | 🟡 prefill prune | ✅ Hermes |

**Bottom line (minimum):** Hermes wins default lean-ness (~3K schema + ~1K guidance).  
**Bottom line (cache):** EdgeCrab wins architecture; Hermes wins activation breadth.

---

## Mode comparison table

| Component | M0 clean-room | | M1 default | | M2 realistic (repo cwd) | |
|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| | **H** | **EC** | **H** | **EC** | **H** | **EC** |
| Tool schemas | ~3.2K | ~4–5K EST | **~15.1K** | ~18K EST | +0 | +0 |
| Stable guidance | ~0.8K | ~1.0K | ~2.0K | **~2.3K** | +coding ~0.9K | +0 |
| Context files | 0 | 0 | 0 | 0 | **~5K** | **~5K** |
| Memory | 0 | 0 | ~0 | ~0 | user | user |
| Skills index | 0 | 0 | ~0.5–2K | ~0.5–2K | grows | grows |
| Volatile stamp | ~50 | ~50 | ~50 | ~50 | ~50 | ~50 |
| **Total (pre-user)** | **~4–6K** | **~7–9K** | **~17–20K** | **~21–24K** | **~22–26K** | **~24–28K** |

---

## Guidance constants — direct diff

| Constant / behavior | Hermes ~tok | EdgeCrab ~tok | Delta (EC−H) |
|---------------------|------------|---------------|--------------|
| Identity | 128 | 138 | +10 |
| Product help block | 140 | 0 | −140 (H only) |
| Task completion (universal) | 192 | **0** | **EC missing** |
| Memory guidance | 356 | 190 | −166 (H longer!) |
| Session search | 46 | 47 | ≈0 |
| Skills guidance | 96 | 97 | ≈0 |
| Steer channel note | 170 | 0 | H only |
| Scheduling / cron prose | 0 | **536** | **+536 EC** |
| Message delivery prose | 0 | **221** | **+221 EC** |
| Vision disambiguation | 0 | **362** | **+362 EC** |
| Task status + progression | 0 | **291** | **+291 EC** |
| File output enforcement | 0 | **185** | **+185 EC** |
| Research-to-file | 0 | **198** | **+198 EC** |
| LSP navigation prose | 0 | **378** | **+378 EC** (when LSP loaded) |
| OpenAI execution (non-Claude) | 673 | 403 | H longer when injected |
| Kanban worker protocol | 1014 | similar when enabled | ≈ |

**Insight:** EdgeCrab traded Hermes's **one** universal completion block for **many** feature-specific blocks. Net effect on default Claude CLI: **+1.5K to +2K tokens** of stable guidance.

---

## Tool surface diff (core lists)

| | Hermes `_HERMES_CORE_TOOLS` | EdgeCrab `CORE_TOOLS` |
|--|------------------------------|------------------------|
| Count on paper | 49 | **64** |
| Active w/ default gates | **35 MEASURED** | ~45–55 EST |
| EC-only examples | — | `web_crawl`, `pdf_to_markdown`, Honcho×6, extra process×7, `skills_hub`, `checkpoint`, split memory |
| H-only examples | `read_terminal`, `browser_cdp`, `browser_dialog`, `image_generate`, kanban in core list | — |

**WHY EC list grew:** Feature parity push (Honcho, HA, MCP, richer browser, web crawl) without tightening default **enabled** set.

---

## Architecture diff (build tier vs API wire)

```
  HERMES (build)                      EDGECRAB (build + wire)
  ──────────────                      ─────────────────────────

  stable ──┐                          stable ──► API sys[0] + cache_control
  context ─┼── ONE string ──► API     dynamic ─► API sys[1] no cache
  volatile ┘
```

| Question | Hermes | EdgeCrab |
|----------|--------|----------|
| Turn-1 minimum | Similar if skills empty | Similar |
| Turn-50 **stable guidance** cost | Hits only if **entire** system string unchanged | Stable block hits even when dynamic changes |
| Turn-50 **skills index** cost | Can cache-read with guidance (same string) | Always full price (dynamic) |
| Skill install (next session rebuild) | Invalidates whole system cache | Invalidates dynamic only |

**≠ who wins:** Hermes maximizes **cached mass** when quiescent; EdgeCrab maximizes **stable prefix survival** when volatile churns. See [006-cache-preservation.md](006-cache-preservation.md).

---

## Default config law

| Setting | Hermes default | EdgeCrab default |
|---------|----------------|------------------|
| `enabled_toolsets` | `None` | `None` |
| `skip_context_files` | `False` | `False` |
| `skip_memory` | `False` | `False` |
| Subagent | skip both | skip both |

**Both products assume: “full agent” unless user opts down.**

---

## AGENTS.md in this workspace (sanity check)

| File | Raw bytes | After 20K trunc ~tok |
|------|-----------|----------------------|
| `edgecrab/AGENTS.md` | 36,483 | ~5,000 |
| `hermes-agent/AGENTS.md` | 69,824 | ~5,000 (cap binds) |

Irony: Hermes repo AGENTS.md is **2× larger** on disk — both hit the same truncation wall. **Minimum context in your own repo is not dogfooding lean prompts.**

---

## Brutal truths (no hedging)

1. **Neither agent is “minimal” out of the box.** ~40–50% of a 128K window can be gone before turn 1 in a configured dev environment.

2. **Hermes's measured 15K schema win is real but modest (~3K tokens).** Not order-of-magnitude.

3. **EdgeCrab's guidance sprawl is self-inflicted** — each feature added a prompt block instead of schema-only discipline + one completion law.

4. **`CORE_TOOLS` comment says ~45; code says 64.** Schema budget already lost internal accountability.

5. **ACP mode is EdgeCrab's worst default** for context — full LSP schemas + LSP guidance without user opt-in.

6. **Prefix cache shipped on EC** — but **Anthropic-only** activation; Hermes covers more routes.

---

## Cache preservation (summary)

Full analysis: [006-cache-preservation.md](006-cache-preservation.md).

| | Hermes | EdgeCrab |
|--|--------|----------|
| System at API | One joined string | Two blocks when cache ON |
| Stable survives date / memory / AGENTS | ❌ | ✅ |
| Skills in cloud cache | ✅ bundled when string static | ❌ always dynamic |
| Provider `cache_control` | OpenRouter, Nous, Qwen, … | Native Anthropic only |
| Local KV hygiene | ✅ | 🟡 |

---

## Cross-refs

- Principles: [001-first-principles.md](001-first-principles.md)
- Inventories: [002-edgecrab-inventory.md](002-edgecrab-inventory.md) · [003-hermes-inventory.md](003-hermes-inventory.md)
- Actions: [005-leverage-plan.md](005-leverage-plan.md)
- Cache: [006-cache-preservation.md](006-cache-preservation.md)
