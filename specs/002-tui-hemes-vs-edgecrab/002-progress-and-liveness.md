# 002 — Progress & Liveness

**First principle:** Users tolerate slow work; they do not tolerate **silent** work.

**Winner on foreground tools: EdgeCrab (decisive).**

---

## Scenario matrix

| Scenario | Hermes | EdgeCrab |
|----------|--------|----------|
| Long `cargo build` | Spinner + elapsed; **no compiler lines** until exit | Placeholder + **last 3 lines @ 5/sec** |
| `execute_code` mid-run | Activity touch only | `TailByteWriter` streaming tail |
| `wait_for_process` poll | Silent + 10s activity touch | **2s heartbeat** + tail snapshot |
| Web search / fetch / crawl | start/complete in UI path | backend attempt + page milestones |
| Browser CDP | start/complete (+ MoA `tool.progress` in tests) | milestone set + wait heartbeat |
| Verbose off | Events gated; minimal status | **`⏳` + tail** via `format_minimal_tool_indicator` |
| Context compression | invisible to user | **`ActivityNotice`** start/done/circuit-breaker |
| Approval wait | overlay + activity touch | overlay + **`ActivityNotice`** |

---

## Hermes progress model

Lifecycle-first pipeline:

1. `tool.generating` → trail “drafting `{name}`…”
2. `tool.start` → active tools + spinners
3. `tool.progress` → preview when emitted
4. `tool.complete` → finalize trail

**Foreground shell:** Python buffers stdout; UI receives **liveness labels**, not bytes. `touch_activity_if_due` fires at most every **10s** with text like `"terminal command running (Ns elapsed)"`.

**Long-run UX:** `useLongRunToolCharms.ts` — first charm after **8s**, max **2 per tool**, **10s** between charms on same tool — pushes to activity via `turnController.pushActivity`.

**Gateway foreground tool progress:** `progress_callback` in `gateway/run.py` with **`_PROGRESS_EDIT_INTERVAL = 1.5`** — throttled message **edits**; often start-focused rather than streaming compiler output.

---

## EdgeCrab progress model

Lifecycle **plus** throttled tail:

1. `ToolGenerating` → `ShelfPhase::GeneratingTool` + spinner
2. `ToolExec` → shelf row + transcript placeholder
3. `ToolProgress` → in-place preview/detail update
4. `ToolDone` → finalize with duration

**Single DRY module:** `tool_progress_tail.rs` — ANSI strip, tail-3, heartbeats, gateway formatters, minimal indicator for verbose-off.

**Long-run UX:** `turn_activity.rs` mirrors Hermes timing:

```10:20:../../crates/edgecrab-cli/src/turn_activity.rs
pub const LONG_RUN_HINT_SECS: u64 = 8;
pub const MAX_LONG_RUN_HINTS_PER_TURN: usize = 4;
pub const MAX_LONG_RUN_HINTS_PER_TOOL: usize = 2;
pub const LONG_RUN_HINT_INTERVAL_SECS: u64 = 10;
```

---

## Background processes

| Surface | Hermes | EdgeCrab |
|---------|--------|----------|
| Shelf inline preview | status poller | **120 chars** (`SHELF_BG_TAIL_CHARS`) |
| Dedicated viewer | RPC `process.list` → **4000 char** tail | `/tail` → **4096 char** (`process_tail_panel.rs`) |
| Gateway while running | `_run_process_watcher` sends **500 char** snippet on new output (`run.py` L12100–12104) | `BackgroundProcessTail` + `bg_tail_chars: 500` default (`gateway/config.rs` L48–61) |
| Gateway on exit | up to **~2000 char** synthetic message (`run.py` L12016–12028) | completion events via `event_processor.rs` |

**Verdict:** Dedicated tail panels — **tie** (4KB). Gateway bg **running** push — **tie** after EC `bg_tail_chars: 500`. Hermes exit synthetic inject is richer for messaging agents.

---

## Stuck-scenario replay

From [../002-terminal-ux-ui/006-stuck-scenarios-playbook.md](../002-terminal-ux-ui/006-stuck-scenarios-playbook.md):

| ID | Hermes feel | EdgeCrab feel |
|----|-------------|---------------|
| S1 cargo build | Frozen spinner | Last compiler lines visible |
| S2 verbose off | Quiet transcript | **`⏳` + tail** |
| S3 bg server | notification poller / watcher | **`📟` monitor** line + `/tail` |
| S4 wait_for_process | Silent poll | 2s heartbeat |
| S7 compression | nothing | ActivityNotice |
| S8 approval | overlay | overlay + notice |
| S12 gateway Telegram | typing + start bubble | typing + throttled tail |

---

## Regressions to avoid

> Do **not** drop live `ToolProgress` tail to match Hermes lifecycle-only model.

That would surrender EdgeCrab’s primary differentiator for terminal-heavy users.

---

## Code anchors

| Behavior | Path |
|----------|------|
| EC minimal indicator | `edgecrab-tools/src/tool_progress_tail.rs` — `format_minimal_tool_indicator` |
| EC wait heartbeat | `edgecrab-tools/src/tools/process.rs` — `wait_for_process` loop |
| EC ActivityNotice | `edgecrab-core/src/conversation.rs` — compression / approval emits |
| Hermes activity touch | `hermes-agent/tools/environments/base.py` — `touch_activity_if_due` |
| Hermes long charms | `hermes-agent/ui-tui/src/app/useLongRunToolCharms.ts` |
| Hermes bg watcher 500 | `hermes-agent/gateway/run.py` ~L12097–12104 |
| EC gateway bg tail | `edgecrab-gateway/src/config.rs` — `bg_tail_chars: 500` |
