# 003 — Core Agent Harness

ReAct loop, goals, steering, compression, completion — the **brain** comparison.

---

## ReAct loop

| Aspect | Hermes (`run_agent.py`) | EdgeCrab (`conversation.rs::execute_loop`) |
|--------|-------------------------|---------------------------------------------|
| Message format | OpenAI-compatible | Same |
| Reasoning field | Stored on assistant msgs | `Message::reasoning` |
| Default max turns | Config-driven | 90 iterations |
| Tool parallel dispatch | Yes | Yes |
| Per-turn mutation footer | Hermes tracking | `mutations.rs` A/M/D footer (spec 002) |
| Context engine hook | `context.engine` plugins | `ContextEngine` trait optional |
| Trajectory save | Config flag | `save_trajectories` |

**Verdict:** **Parity** with EdgeCrab extras: **file-mutation verifier**, **shadow judge** hook at completion.

---

## System prompt assembly

Both assemble ~12 sources; both **forbid mid-conversation system prompt rebuild** (cache safety).

| Source | Hermes | EdgeCrab |
|--------|--------|----------|
| Identity / SOUL | Yes | Yes |
| Platform hints | Yes | Yes |
| Timestamp | Dynamic zone | Dynamic zone only |
| AGENTS.md, .cursorrules, etc. | Yes + injection scan | Yes + injection scan |
| MEMORY.md, USER.md | Yes | Yes |
| Skills index | Yes | Yes |
| Session search guidance | Yes | Yes |
| Tool-specific guidance | Yes | Yes (+ LSP, browser, computer use) |
| Anthropic prefix cache TTL | `prompt_caching.cache_ttl` | `cache.prompt_prefix.ttl` |

EdgeCrab explicitly splits **stable** vs **dynamic** blocks in `prompt_builder.rs` for cross-session Anthropic cache hits.

**Verdict:** **Parity (A)** — EdgeCrab documents cache zones more explicitly.

---

## Compression

| Feature | Hermes | EdgeCrab |
|---------|--------|----------|
| Threshold trigger | 50% default | 50% default |
| Gateway 85% hygiene | Yes | Pressure warning |
| LLM 8-section summary | Yes | Yes (`SUMMARY_PREFIX`) |
| Structural fallback | Yes | Yes |
| Prune tool outputs | Yes | Yes |
| `protect_last_n` | Yes (20) | Yes (20) |
| Manual `/compress` | `[here N]`, `focus` | `/compress` |
| Spill large tool results | Yes | Yes (disk spill) |

**Verdict:** **Parity (A)**.

---

## Persistent goals (Ralph loop)

| Feature | Hermes | EdgeCrab |
|---------|--------|----------|
| `/goal`, `/subgoal` | Yes | Yes |
| Storage | `SessionDB.state_meta` | SQLite `session_goals` / `session_subgoals` |
| Injected as user message each turn | Yes | Yes (never mutates cached system) |
| Goal judge model | `auxiliary.goal_judge` | Same |
| Turn budget | `goals.max_turns` (20) | Same |
| `/goal pause\|resume\|clear\|status` | Yes | Yes |
| `/done` mark subgoal complete | **No** | **Yes** |

**Verdict:** **EdgeCrab leads slightly** — `/done` + dedicated goal tables outside message history.

---

## Mission steering vs `/steer`

| | Hermes | EdgeCrab |
|---|--------|----------|
| Mechanism | `/steer <text>` — inject after next tool | Typed steers: **Hint**, **Redirect**, **Stop** |
| Interrupt running tool | `/busy interrupt` mode | **Stop** steer + cancel token |
| Gateway second message | `busy` modes: queue/steer/interrupt | `second_message_mode`: queue/steer/interrupt |
| TUI UX | Text command | Ctrl+S overlay + status chip ⛵ |
| Injection scan | Partial | `steering.rs` scan |

**Verdict:** **EdgeCrab leads** — richer steering model; Hermes covers 80% with `/steer` + `/busy`.

---

## Completion assessment

What happens when the model stops calling tools?

| Check | Hermes | EdgeCrab |
|-------|--------|----------|
| Pending clarify | Yes | Yes |
| Pending approval | Yes | Yes |
| Open todos | Yes | Yes |
| Child delegates running | Yes | Yes |
| Goal loop continuation | Yes | Yes |
| **Shadow judge** (LLM verifies done) | **No** | **Yes** (`shadow_judge.rs`) |
| Verification markers | Partial | `completion_assessor.rs` |

**Verdict:** **EdgeCrab leads** on automated "are we really done?" — controversial (extra LLM cost/latency) but unique.

---

## Delegation & subagents

| | Hermes | EdgeCrab |
|---|--------|----------|
| Tool | `delegate_task` | `delegate_task` |
| Child agent | New `AIAgent` | `CoreSubAgentRunner` |
| Blocked child tools | delegate, clarify, memory, send_message, execute_code | Configurable depth + policy |
| Depth limit | `delegation.max_spawn_depth` | Same concept |
| TUI monitor | `/agents` (React overlay) | `/agents` (ratatui) + Gantt + replay |
| **Kanban queue** | **9 tools, SQLite, OS processes** | **None** (gap 007) |
| Global spawn pause | Partial | Yes (`delegation_state.rs`) |
| Per-subagent kill | Yes | Yes (`x`/`X` in overlay) |
| Disk spawn tree | Partial | `spawn_tree_store.rs` |

**Verdict:** **Hermes leads operational model** (kanban = durable multi-agent ops). **EdgeCrab leads TUI control plane** (replay, Gantt, pause) for in-process delegates only.

---

## Background work

| Mode | Hermes | EdgeCrab |
|------|--------|----------|
| `/background` / `/btw` | Yes | `/background`, `/btw` |
| `/queue` | Yes | Yes + queued panel UX |
| Cron-isolated agent | Fresh agent per tick | Fresh agent per tick |

**Verdict:** **Parity**.

---

## Checkpoints & rollback

| | Hermes | EdgeCrab |
|---|--------|----------|
| Implementation | `checkpoint_manager.py` shadow git | `tools/checkpoint/` v2 |
| User interface | `/rollback [N]` | `/rollback` + `checkpoint` tool |
| Default enabled | **Off** (`checkpoints.enabled: false`) | Config-driven |
| Auto snapshot per turn | Hermes behavior | Before file mutations |
| `/snapshot` (config state) | Yes | No direct equivalent |

**Verdict:** **Parity on filesystem rollback**; Hermes adds **config snapshot** separate from git checkpoints.

---

## Grades (core harness)

| Dimension | Hermes | EdgeCrab |
|-----------|--------|----------|
| ReAct correctness | A | A |
| Compression | A | A |
| Goals / Ralph loop | A | A+ |
| Steering | B+ | A |
| Completion truth | B | A− (shadow judge) |
| Multi-agent ops | A (kanban) | B (delegate only) |
| Checkpoints | A | A |

Cross-ref: [001-gap-analysis 001/006/007](../001-gap-analysis-v14/999-roadmap.md)
