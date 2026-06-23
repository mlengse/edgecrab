# 001 — First Principles: What Is “Minimum Context”?

## The only question that matters

> **On the first API call of a cold session, how many tokens does the harness spend before the user's intent enters the model?**

That spend is not “overhead” in the abstract — it is **competition** with:

- The user's actual task description
- Tool results on later turns
- Room left for reasoning on long files

An agent harness that burns 25K tokens on “hello” has already consumed ~20% of a 128K window **before doing any work**.

---

## Decompose the first turn (physics, not product marketing)

```
                    FIRST LLM REQUEST
    ┌──────────────────────────────────────────────────────────┐
    │                                                          │
    │   ┌─────────────────────────────────────────────┐        │
    │   │  SYSTEM PROMPT (string or block array)       │        │
    │   │  ├─ stable identity & behavioral law         │        │
    │   │  ├─ tool-conditioned guidance (if tool X…)   │        │
    │   │  ├─ project context files (AGENTS.md…)       │        │
    │   │  ├─ memory / USER.md                         │        │
    │   │  ├─ skills index                             │        │
    │   │  └─ volatile stamp (date, session, model)    │        │
    │   └─────────────────────────────────────────────┘        │
    │                          +                                 │
    │   ┌─────────────────────────────────────────────┐        │
    │   │  TOOL SCHEMAS (`tools[]` in API)             │  ◄───┐ │
    │   │  One JSON blob per tool: name, description,  │      │ │
    │   │  parameters, enums, examples, strict flags   │      │ │
    │   └─────────────────────────────────────────────┘      │ │
    │                          +                               │ │
    │   ┌─────────────────────────────────────────────┐        │ │
    │   │  USER MESSAGE                                │        │ │
    │   └─────────────────────────────────────────────┘        │ │
    │                                                          │ │
    └──────────────────────────────────────────────────────────┘ │
                                                                 │
              OFTEN THE LARGEST SINGLE BUCKET ───────────────────┘
              (typically 12–20K tokens at default settings)
```

### Principle 1 — Schemas are not optional metadata

The model reads tool definitions on **every** turn they are attached. Shrinking the system prompt while leaving 50 tools in `tools[]` is **whack-a-mole**.

**WHY:** Provider billing counts input tokens for schemas identically to prose. There is no “tools are free” lane.

### Principle 2 — “Stable” vs “dynamic” affects cost, not minimum

Anthropic prefix caching makes stable bytes cheaper **on turn 2+**. On turn 1, **everything is a write**. Minimum context and cache architecture are related but not the same problem.

```
  Turn 1:   pay full price for ALL bytes (system + tools + user)
  Turn 2+:  stable prefix may cache-hit IF bytes unchanged

  Minimum-context work  → shrink turn-1 floor
  Cache work            → shrink turn-2+ marginal cost
```

See [specs/effective_prompt/06-composition-order.md](../effective_prompt/06-composition-order.md).

**EdgeCrab (shipped):** At the API wire, stable and dynamic are **separate** `ChatMessage::system` blocks when `cached_stable_prompt` + Anthropic cache are active — only the stable block gets `cache_control` (`conversation.rs` `build_chat_messages_blocks`). That preserves stable KV when date, memory, or AGENTS.md change. Details: [006-cache-preservation.md](006-cache-preservation.md).

**Hermes:** Build tiers (`stable` / `context` / `volatile`) are joined into **one** system string before `apply_anthropic_cache_control` — simpler storage, coarser invalidation.

### Principle 3 — Gating beats prose

The cheapest guidance is **absence**. Both harnesses gate some blocks on `valid_tool_names` / `has_tool()`. The winner removes:

1. The tool from schemas **and**
2. The guidance that references it

EdgeCrab is **better** at (2) for many blocks (`prompt_builder.rs` `has_tool` gates). Both are **weak** at (1) when `enabled_toolsets` is unset.

### Principle 4 — Default config IS the product

Users rarely set `EDGECRAB_SKIP_CONTEXT_FILES` or Hermes `skip_context_files`. **The default path is the honest comparison.**

If your default ships 64 tool names and 13K tokens of guidance, your product **is** a context-heavy agent — regardless of minimal mode existing in a spec.

### Principle 5 — Context files are unbounded until truncated

Both cap individual files at **20,000 chars** (~5K tokens each) with head/tail truncation:

| Engine | Constant | File |
|--------|----------|------|
| EdgeCrab | `CONTEXT_FILE_MAX_CHARS = 20_000` | `prompt_builder.rs:716` |
| Hermes | `CONTEXT_FILE_MAX_CHARS = 20_000` | `prompt_builder.py:947` |

**WHY head/tail:** Instructions often live at top; signatures/licenses at bottom. Middle truncation loses both.

A repo with AGENTS.md + SOUL.md + `.cursor/rules/*.mdc` can still add **15K+ tokens** in the dynamic/context tier even after truncation.

---

## Three measurement modes (use consistently)

| Mode | Config sketch | What it tells you |
|------|---------------|-------------------|
| **M0 Clean-room** | `skip_context_files`, `skip_memory`, `enabled_toolsets: [minimal]` | Engineering floor — “how small could we be?” |
| **M1 Default harness** | Fresh config, `enabled_toolsets: None`, empty memory | What power users actually get |
| **M2 Realistic dev** | M1 + AGENTS.md in cwd + 5–20 skills installed | What Cursor/CLI users hit in this repo |

**Do not compare M0 for one engine against M2 for the other.** That is how spec documents lie.

---

## The leverage stack (ordered by first principles)

```
  ROI ↑
  │
  │  ┌─────────────────────────────────────────┐
  │  │ L1  Default toolset policy              │  ← biggest single lever
  │  └─────────────────────────────────────────┘
  │  ┌─────────────────────────────────────────┐
  │  │ L2  Schema slimming (descriptions)      │
  │  └─────────────────────────────────────────┘
  │  ┌─────────────────────────────────────────┐
  │  │ L3  Lazy / on-demand tool registration  │
  │  └─────────────────────────────────────────┘
  │  ┌─────────────────────────────────────────┐
  │  │ L4  Guidance consolidation              │  ← EdgeCrab pain point
  │  └─────────────────────────────────────────┘
  │  ┌─────────────────────────────────────────┐
  │  │ L5  Context file discovery scope        │
  │  └─────────────────────────────────────────┘
  │  ┌─────────────────────────────────────────┐
  │  │ L6  Skills index compaction             │
  │  └─────────────────────────────────────────┘
  │  ┌─────────────────────────────────────────┐
  │  │ L7  Cache tier placement                │  ← cost, not minimum
  │  └─────────────────────────────────────────┘
  └──────────────────────────────────────────────► effort
```

Details: [005-leverage-plan.md](005-leverage-plan.md) · cache depth: [006-cache-preservation.md](006-cache-preservation.md).

---

## What this spec set does NOT claim

- **Not** a cold-start latency benchmark (see [022-cold-start-perf](../001-gap-analysis-v14/022-cold-start-perf/)).
- **Not** compression behavior mid-session.
- **Not** gateway-specific platform hints (Telegram vs CLI differ).
- **Not** MCP server tools (unbounded — user config dominates).

---

## Next

- EdgeCrab assembly law: [002-edgecrab-inventory.md](002-edgecrab-inventory.md)
- Hermes assembly law: [003-hermes-inventory.md](003-hermes-inventory.md)
- Comparison table: [004-comparison-matrix.md](004-comparison-matrix.md)
- Prefix / KV cache: [006-cache-preservation.md](006-cache-preservation.md)
