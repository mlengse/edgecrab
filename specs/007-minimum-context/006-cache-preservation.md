# 006 — KV / Prefix Cache Preservation (EdgeCrab vs Hermes)

**Parent:** [007-minimum-context](README.md) · **Related:** [004-comparison-matrix](004-comparison-matrix.md) · [005-leverage-plan](005-leverage-plan.md)

**Code is law.** This doc double-checks how each harness keeps provider prefix caches
(and local KV caches) warm across turns. It complements
[001-first-principles.md](001-first-principles.md) (minimum bytes) with **cache hit rate**
(how many of those bytes are cheap on turn 2+).

**EdgeCrab cache status:** Shipped — see [004-prompt-prefix-cache/plan.md](../001-gap-analysis-v14/004-prompt-prefix-cache/plan.md). Supersedes stale `003-edgecrab-current-state.md`.

---

## Two different “caches” (do not conflate)

```
  ┌─────────────────────────────────────────────────────────────────┐
  │  A. CLOUD PROMPT CACHE (Anthropic / OpenRouter / Qwen / …)      │
  │     cache_control breakpoints → cache_read_input_tokens         │
  │     Billing: ~10× cheaper on cache hits                         │
  └─────────────────────────────────────────────────────────────────┘

  ┌─────────────────────────────────────────────────────────────────┐
  │  B. LOCAL KV CACHE (llama.cpp, vLLM, Ollama, LM Studio)        │
  │     Identical byte prefix across requests → reuse GPU KV         │
  │     No cache_control — purely prefix matching on messages       │
  └─────────────────────────────────────────────────────────────────┘
```

Minimum-context work shrinks **A and B** (fewer bytes).  
This doc asks: **given the same bytes, who preserves prefix stability better?**

---

## Anthropic / cloud prompt cache — architecture comparison

### EdgeCrab: **two system blocks at the API boundary**

```
  API messages (when cache ON + build_blocks path):

  ┌──────────────────────────────────────────────────────────────┐
  │ ChatMessage::system(STABLE)                                  │
  │   identity + tool-gated guidance (~2.6–3.2K tok)             │
  │   cache_control: { type: ephemeral, ttl: "1h" }  ◄── BP #1   │
  ├──────────────────────────────────────────────────────────────┤
  │ ChatMessage::system(DYNAMIC)                                 │
  │   date + session + AGENTS.md + memory + skills               │
  │   NO cache_control — intentionally uncached                  │
  ├──────────────────────────────────────────────────────────────┤
  │ … conversation history …                                     │
  │   apply_cache_control → last N user msgs ◄── BP #2–4         │
  └──────────────────────────────────────────────────────────────┘

  Law: conversation.rs build_chat_messages_blocks() ~3076
       prompt_builder.rs build_blocks() stable vs dynamic split
       SessionState.cached_stable_prompt set at build ~967
```

**WHY this wins:** Anthropic invalidates a cache block when **any byte** in that
block changes. Splitting means:

| Change | EdgeCrab stable cache | Hermes (single system string) |
|--------|----------------------|------------------------------|
| New calendar day (date line) | **HIT** — date is in dynamic block | **MISS** — date appended to same string |
| AGENTS.md edit | **HIT** on stable | **MISS** on entire system prompt |
| Memory growth next session | **HIT** on stable | **MISS** |
| Skills index update | **HIT** on stable; pay full price for skills in dynamic | **MISS** on entire blob |
| Tool-gated guidance unchanged | **HIT** | **HIT** (if nothing else changed) |

**Trade-off:** EdgeCrab **never** cloud-caches the skills index or memory — it
re-pays ~0.5–2K dynamic tokens every turn. Hermes **can** cache skills+guidance
together **if** the entire joined string is byte-identical (same day, same memory,
same context files).

**Net:** EdgeCrab optimizes **stable behavioral law** cache survival. Hermes
optimizes **maximum cached mass** when the session is quiescent.

---

### Hermes: **three tiers in code, one tier at the API**

```
  build_system_prompt_parts()          build_system_prompt()
  ┌─────────────────┐                  ┌─────────────────────────┐
  │ stable          │                  │ ONE system string:      │
  │  + skills INDEX │ ─── join ──────► │ stable + context +      │
  │ context         │                  │ volatile                │
  │ volatile        │                  └───────────┬─────────────┘
  └─────────────────┘                              │
                                                   ▼
                                    api_messages[0] role=system
                                    cache_control on LAST part only
                                    (prompt_caching.py system_and_3)

  Law: agent/system_prompt.py ~62–404
       agent/prompt_caching.py apply_anthropic_cache_control()
       agent/conversation_loop.py ~689–694
```

The `stable` / `context` / `volatile` split is **documentation and build order**,
not separate API blocks. At the wire, Hermes still sends **one** system message.

**WHY Hermes did this:** Simpler provider integration; one `_cached_system_prompt`
string restored verbatim from SQLite for gateway per-turn agents
(`conversation_loop.py` `_restore_or_build_system_prompt`).

**Cost:** Any volatile-byte change (date rollover, memory injection, context files)
invalidates cache for **skills + guidance + identity** bundled in the same string.

**Mitigations Hermes already ships:**

- Date-only timestamp (not minute) — `system_prompt.py:365–371`, PR #20451
- Memory **frozen snapshot** at session start — never mid-session mutation
  (`tools/memory_tool.py:119`, docs)
- System prompt **not rebuilt** mid-session except `/compress`
- Session DB stores exact `system_prompt` for prefix reuse across gateway turns

EdgeCrab ported date-only policy (`prompt_builder.rs:1304–1307`).

---

## Breakpoint budget (both: system + rolling 3)

Anthropic allows **4** `cache_control` markers per request.

```
  EdgeCrab (blocks path)                Hermes (system_and_3)
  ─────────────────────               ─────────────────────
  BP1: stable system block              BP1: system message (whole string)
  BP2–4: last 3 user messages          BP2–4: last 3 non-system messages
        (apply_cache_control,                 (apply_anthropic_cache_control)
         cache_system_prompt: false)
```

Conversation rolling cache behavior is **≈ parity**. Difference is **what BP1 covers**.

---

## Provider coverage (Hermes leads)

| Provider route | Hermes `anthropic_prompt_cache_policy()` | EdgeCrab `provider_supports_prompt_caching()` |
|----------------|------------------------------------------|-----------------------------------------------|
| Native Anthropic | ✅ native layout | ✅ (`provider.name() == "anthropic"`) |
| OpenRouter Claude | ✅ envelope layout | ❌ not enabled |
| Nous Portal Claude/Qwen | ✅ | ❌ |
| Anthropic-wire third parties (MiniMax, …) | ✅ | ❌ |
| OpenCode / Alibaba Qwen | ✅ | ❌ |
| LM Studio / Ollama local | N/A (no cache_control) | N/A |

**Law:** `agent/agent_runtime_helpers.py:1206–1308` vs `conversation.rs:3162–3180`

**Brutal truth:** EdgeCrab implemented the **better split** but only wires it for
**native Anthropic**. Hermes applies caching across many routes users actually use.

EdgeCrab config:

```yaml
cache:
  prompt_prefix:
    enabled: true    # default
    ttl: "1h"        # default — cross-session stable block
```

Requires **also** `model_config.prompt_caching: true` (legacy flag) AND
`cached_stable_prompt` populated via `build_blocks()`.

---

## When the two-block path is NOT used (EdgeCrab gaps)

```rust
// conversation.rs build_api_chat_messages()
match (session.cached_stable_prompt.as_deref(), cache_cfg) {
    (Some(stable), Some(cfg)) => build_chat_messages_blocks(...),
    _ => build_chat_messages(combined, ...),  // single system blob — Hermes-like
}
```

Falls back to single-block (cache on **entire** combined prompt) when:

1. Explicit `system_prompt` override without stable split (`cached_stable_prompt` None)
2. `prompt_caching` disabled or non-anthropic provider
3. `cache.prompt_prefix.enabled: false`

**Action:** Ensure `build_blocks()` always runs for normal sessions (already true in
`conversation.rs` ~934–968).

---

## Cache invalidation matrix

| Event | EdgeCrab stable BP | EdgeCrab dynamic | Hermes system BP | Conversation BPs |
|-------|-------------------|------------------|------------------|-------------------|
| Turn N+1 same session | HIT | unchanged if no file/memory edits | HIT if bytes identical | rolling window re-establishes |
| `/compress` | Rebuild — MISS then rewrite | rebuilt | `_invalidate_system_prompt` — MISS | history reshaped — MISS mid-thread |
| `/reload-mcp` | stable unchanged | unchanged | stable unchanged | **tools[] change** — prefix after tools may MISS |
| Memory write mid-session | HIT (not in prompt until rebuild) | unchanged (frozen until rebuild) | HIT (frozen snapshot) | — |
| `/goal` inject | HIT — injected as **user** msg, not system | HIT | HIT | goal in user tail — rolling cache absorbs |
| `/skills install` | HIT until invalidate + rebuild | changes on rebuild | no rebuild until compress — **stale skills in prompt** | — |
| Personality addon change | HIT — addon in dynamic | changes | in combined string — **MISS** | — |
| Model switch | MISS (model change) | MISS | MISS | MISS |

**Goals (EdgeCrab advantage):** `render_goal_block()` appended as ephemeral user
message each turn — does **not** touch `cached_system_prompt`
(`conversation.rs:1861–1910`, test `execute_loop_injects_goal_block_without_persisting`).

**Steering (Hermes):** Mid-turn steers go to tool results, not system — both preserve
system prefix. Hermes documents trust marker in stable tier (`STEER_CHANNEL_NOTE`).

---

## Local KV cache (llama.cpp / vLLM / Ollama)

### Hermes: explicit prefix hygiene

```text
  conversation_loop.py ~712–743 (on api_messages copy):
    • strip() string contents
    • json.dumps tool args with sort_keys=True, compact separators
  → "bit-perfect prefixes across turns" (comment in code)
```

**WHY:** Local servers reuse KV when the serialized prompt prefix matches **exactly**.
Whitespace or JSON key order drift → full recompute.

### EdgeCrab: different lever — prefill prune

Spec 014 / `local_provider_policy`: drop tool schemas (and optionally trim messages)
when local context is tight — **reduces bytes**, not normalization.

**No equivalent** of Hermes's per-turn tool-call JSON canonicalization found in
`edgecrab-core`.

| Local inference | Hermes | EdgeCrab |
|-----------------|--------|----------|
| Prefix byte stability | ✅ normalization pass | 🟡 no dedicated pass |
| Tool schema pressure | carries full schemas | ✅ prefill prune can drop tools |
| System stable split | single string | two blocks still help prefix if provider sees concatenation — **depends on edgequake-llm wire format** |

**Follow-up:** Port Hermes JSON/key-order normalization to `append_conversation_messages`
for `lmstudio`/`ollama` providers.

---

## Cross-session 1h cache (the “KV cache” users feel as $ savings)

```
  Session 1 (Mon 10:00)          Session 2 (Mon 15:00)         Session 3 (Tue 09:00)
  ─────────────────────          ─────────────────────         ─────────────────────

  EdgeCrab stable block          Same toolset + model           Date in dynamic changes
  WRITE 1h tier                  READ stable @ 0.3×             Stable STILL HIT (1h TTL)
  Dynamic paid full              Dynamic paid full              Dynamic paid full

  Hermes full system string      Same if ALL bytes match        Date in volatile →
  WRITE if first in hour         READ if identical              ENTIRE system MISS
                                 (includes date if day changed)
```

**EdgeCrab wins cross-session stable reuse** when users take breaks **within the same
calendar day** or when memory/context differ but guidance constants do not.

**Hermes wins** when the **entire** prompt is identical (no date change, no memory
delta) — cached chunk is **larger** (includes skills index).

---

## Scorecard

| Dimension | Winner | Notes |
|-----------|--------|-------|
| API stable/dynamic split | **EdgeCrab** | Real two-block wire format |
| Stable prefix survival on date/memory/context churn | **EdgeCrab** | Dynamic isolation |
| Cached mass per hit (skills in BP1) | **Hermes** | Skills in stable string |
| Provider breadth | **Hermes** | OpenRouter, Qwen, MiniMax, … |
| Conversation rolling cache | **≈ tie** | system_and_3 vs apply_cache_control |
| Mid-session system immutability | **≈ tie** | both freeze prompt; memory snapshot |
| Goals / steering without system mutation | **≈ tie** | both inject outside system |
| Local KV prefix hygiene | **Hermes** | JSON sort + strip |
| Local context pressure | **EdgeCrab** | prefill prune |
| Implementation freshness | **EdgeCrab** | old gap doc 003 was stale — cache shipped per `004-prompt-prefix-cache/plan.md` |

---

## ASCII: ideal combined architecture (leverage)

```
  TARGET (EdgeCrab + Hermes best-of)

  ┌─ system[0] STABLE + cache_control 1h ─────────────────────┐
  │  identity + tool-gated guidance only                       │
  └────────────────────────────────────────────────────────────┘
  ┌─ system[1] SEMI-STABLE + cache_control 5m ────────────────┐  ← optional tier
  │  skills INDEX (changes rarely)                             │
  └────────────────────────────────────────────────────────────┘
  ┌─ system[2] DYNAMIC (no cache) ────────────────────────────┐
  │  date + memory + AGENTS.md                                 │
  └────────────────────────────────────────────────────────────┘
  … messages + rolling 3 breakpoints …

  + anthropic_prompt_cache_policy() breadth on EdgeCrab providers
  + Hermes local JSON normalization on EdgeCrab ollama/lmstudio path
```

Uses 3 system blocks = 3 breakpoints; leaves 1 for rolling messages — fits Anthropic's
limit of 4 if skills tier shares BP with stable or uses 5m TTL on semi-stable only.

---

## Cross-refs

- [004-comparison-matrix.md](004-comparison-matrix.md) — token sizes
- [005-leverage-plan.md](005-leverage-plan.md) — P4 cache polish items
- [specs/effective_prompt/02-cache-architecture.md](../effective_prompt/02-cache-architecture.md)
- [specs/001-gap-analysis-v14/004-prompt-prefix-cache/plan.md](../001-gap-analysis-v14/004-prompt-prefix-cache/plan.md) — shipped status
- Hermes: `website/docs/developer-guide/context-compression-and-caching.md`

**Stale doc warning:** `004-prompt-prefix-cache/003-edgecrab-current-state.md` predates
Phase 2 — treat **this file + plan.md** as current for EdgeCrab cache law.

---

## Document index

| File | Role |
|------|------|
| [README.md](README.md) | Dual verdict (minimum + cache) |
| [002-edgecrab-inventory.md](002-edgecrab-inventory.md) | EC wire format + config gates |
| [003-hermes-inventory.md](003-hermes-inventory.md) | Hermes system_and_3 + provider policy |
| [004-comparison-matrix.md](004-comparison-matrix.md) | Scorecard rows |
| [005-leverage-plan.md](005-leverage-plan.md) | L4.3–L4.5 implementation items |
