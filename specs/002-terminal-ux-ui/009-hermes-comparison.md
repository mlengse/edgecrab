# 009 — Hermes-Agent Comparison (TUI & Progression)

Side-by-side evaluation of **EdgeCrab** (`feat/terminal-ux-live-progress`) vs **Hermes-agent** (`/Users/raphaelmansuy/Github/03-working/hermes-agent`) for terminal visibility, tool progression, and “never feel stuck” UX.

**Method:** Code survey of both repos (June 2026). Grades use the same scale as [005-honest-assessment.md](005-honest-assessment.md): **S** meets bar · **P** partial · **F** fails.

---

## Executive summary

| Product | UI stack | Progress philosophy | Overall (visibility / liveness) |
|---------|----------|---------------------|----------------------------------|
| **Hermes** | Ink/React TUI + prompt_toolkit CLI + Python `tui_gateway` | **Lifecycle-first** — tool start/complete, spinners, status bubbles; foreground shell output **not** streamed to UI | **B+** / **B** |
| **EdgeCrab** | ratatui TUI (Rust) + `StreamEvent` bridge | **Lifecycle + throttled tail + activity shelf** — start/done model **plus** last-3-line stdout, `/details` sections, persisted disclosure | **A** / **A** |

**Headline (June 2026, post-shelf pass):** EdgeCrab **matches or exceeds Hermes on shelf semantics** (`/details`, activity feed, sub-agent tool counts) while retaining **live foreground tail** as the terminal-heavy differentiator. Remaining Hermes lead is **Ink visual craft** (sparklines, heat colors), not observability.

---

## Architecture comparison

```text
Hermes (modern TUI)
  Ink UI (Node) ←JSON-RPC→ tui_gateway (Python) ←callbacks← agent/tool_executor
  Events: tool.start | tool.complete | tool.generating | reasoning.delta | status.update
  Foreground terminal: output buffered until exit; touch_activity_if_due @ 10s (gateway keepalive only)

EdgeCrab (TUI)
  ratatui App ←AgentResponse← forward_stream_event_to_tui ← StreamEvent ← conversation loop
  Events: ToolExec | ToolProgress | ToolDone | ActivityNotice | BackgroundProcessTail | …
  Foreground terminal: ToolProgressTail → last 3 lines @ ≤5/sec into placeholder line
```

| Aspect | Hermes | EdgeCrab |
|--------|--------|----------|
| TUI framework | Ink/React (`ui-tui/`) | ratatui (`edgecrab-cli/src/app.rs`) |
| Agent↔UI boundary | `tui_gateway/server.py` JSON events | In-process `StreamEvent` + mpsc |
| Progress config | `display.tool_progress`: off/new/all/verbose | `ToolProgressMode`: same four modes |
| Shared formatter module | Scattered (`display.py`, `limits.ts`, `run.py`) | **`tool_progress_tail.rs`** (single source) |

---

## Dimension-by-dimension matrix

| Dimension | Hermes | EdgeCrab | Winner |
|-----------|--------|----------|--------|
| **Tool lifecycle (start/done)** | `tool.start` / `tool.complete`; in-place “thinking shelf” with active tools + elapsed | `ToolExec` → in-place placeholder → `ToolDone` upgrade + duration | **Tie (A−)** |
| **Foreground terminal mid-run** | Spinner/static preview only; `touch_activity_if_due` every **10s** (no stdout to UI) [`base.py` L687] | **Last 3 lines @ ≤5/sec** via `ToolProgress` [`tool_progress_tail.rs`] | **EdgeCrab (A− vs P)** |
| **execute_code mid-run** | Same batch-wait + activity touch [`code_execution_tool.py`] | `TailByteWriter` streaming tail | **EdgeCrab** |
| **wait_for_process** | Blocks; activity touch only | **2s heartbeat** + tail snapshot [`process.rs`] | **EdgeCrab** |
| **Web search/fetch/crawl** | Start/complete only (no chain milestones in UI path) | Backend attempt + page milestones | **EdgeCrab** |
| **Browser CDP** | Start/complete; MoA emits `tool.progress` in tests | Full milestone set + wait heartbeat + vision analyze step | **EdgeCrab** |
| **Background processes (CLI/TUI)** | `ProcessRegistry` buffer; Ink **`process.list`** with **4000 char** tail [`server.py` ~L7569]; in-place `status.update` + notification poller | In-scrollback **`📟` monitor line**; tail-3 throttled [`bg_process_lines`] | **Hermes (richer tail)** |
| **Background processes (gateway)** | **`_run_process_watcher`**: **500 char** updates while running, **1000–2000** on exit [`run.py` ~L12079] | Throttled status snippets (280 char cap) | **Hermes** |
| **Parallel tools** | Active tools array in turn controller; UI batch 16ms | Per-`tool_call_id` placeholders + `+N more` status; progress follows latest reporter | **Tie (B)** |
| **Verbose-off / minimal** | Events gated off; spinner hidden | Dim **`⏳` line** still updates with tail | **EdgeCrab** |
| **Verbose mode depth** | Full args + result in scrollback; 800 char / 12 line trail cap [`limits.ts`] | Extra detail lines + Ctrl+Shift+T expand (terminal/execute_code/browser_snapshot) | **Hermes (slightly)** |
| **Tool arg streaming** | **`tool.generating`** while model drafts JSON | No equivalent (ToolCallDelta internal only) | **Hermes** |
| **Reasoning visibility** | `reasoning.delta` / shelf integration | Opt-in `/reasoning show`; ghost line | **Hermes (Ink shelf)** |
| **Compression UX** | tracing + system note to model (invisible) | **`ActivityNotice`** start/done/circuit-breaker | **EdgeCrab** |
| **Approval waiting** | Activity touch @ 10s [`approval.py`]; overlay | **`ActivityNotice`** + overlay | **EdgeCrab (explicit notice)** |
| **Gateway tool progress** | **Start-only** editable bubbles; 1.5s edit throttle [`_PROGRESS_EDIT_INTERVAL`] | Start + **throttled tail** status | **EdgeCrab (mid-run)** |
| **Remote SSH/Modal** | Batch wait + activity touch | Start milestone + tail on complete | **EdgeCrab (start signal)** |
| **UI polish / chrome** | Braille spinners, long-run charms (8s+), desktop app, web PTY | Width-adaptive tool lines, skin engine, mission steering overlay | **Hermes (Ink)** |
| **SDK / API events** | SSE `hermes.tool.progress` lifecycle | Python/Node `tool_progress`, `activity_notice`, bg process | **Tie (B−)** |

---

## What Hermes does better

1. **Ink “thinking shelf”** — active tools, reasoning stream, and sub-agent tree in one animated panel (`ui-tui/src/components/thinking.tsx`, `turnController.ts`).
2. **Dedicated background process viewer** — RPC `process.list` exposes **4000 characters** of rolling buffer for inline terminal viewer (`tui_gateway/server.py`).
3. **Gateway background push** — messaging platforms get **500 char** running updates and **1000–2000 char** completion snippets (`gateway/run.py` `_run_process_watcher`).
4. **`tool.generating`** — user sees “preparing `{tool}`…” while the model streams tool-call JSON (`createGatewayEventHandler.ts`).
5. **Long-run charms** — playful nudge after 8s on stuck tools (`useLongRunToolCharms.ts`).
6. **Per-platform defaults** — Telegram/Slack often `tool_progress: off`; Discord `all` (`gateway/display_config.py`).

## What EdgeCrab does better

1. **Live foreground shell tail** — Hermes explicitly does **not** wire stdout to UI during `_wait_for_process`; EdgeCrab does via `ToolProgressTail` (the largest “stuck during cargo build” fix).
2. **Unified progress module** — `tool_progress_tail.rs`: throttle, ANSI strip, tail-3, gateway formatters, compression/approval notices, remote start milestone (DRY across tools + gateway).
3. **Mid-run milestones** — web backend chain, browser CDP steps, `wait_for_process` / `browser_wait_for` heartbeats.
4. **Verbose-off liveness** — minimal `⏳` indicator still receives tail updates (Hermes gates events off entirely).
5. **Explicit system notices** — compression start/done/circuit-breaker, approval waiting (`ActivityNotice`).
6. **Single-binary TUI** — no Node+Python gateway process for CLI; lower moving parts than `hermes --tui`.

---

## Hermes code anchors (verified)

| Behavior | Path |
|----------|------|
| No stdout stream during foreground wait | `hermes-agent/tools/environments/base.py` — `_wait_for_process` loop, `touch_activity_if_due` L687 |
| TUI gateway skips duplicate tool.started progress | `hermes-agent/tui_gateway/server.py` — `_on_tool_progress` L2631–2645 |
| Tool lifecycle callbacks | `hermes-agent/agent/tool_executor.py` — `tool_progress_callback("tool.started"…)` |
| Process buffer + watch limits | `hermes-agent/tools/process_registry.py` — 200KB buffer, 15s watch interval |
| Gateway progress (start-focused) | `hermes-agent/gateway/run.py` — `progress_callback` ~L13220 |
| Tool progress modes | `hermes-agent/gateway/display_config.py` |
| Ink event handler | `hermes-agent/ui-tui/src/app/createGatewayEventHandler.ts` |

## EdgeCrab code anchors

| Behavior | Path |
|----------|------|
| Tail/throttle/formatters | `edgecrab/crates/edgecrab-tools/src/tool_progress_tail.rs` |
| StreamEvent bridge | `edgecrab/crates/edgecrab-core/src/conversation.rs` — `make_tool_progress_tx`, `forward_process_watch_event` |
| TUI in-place updates | `edgecrab/crates/edgecrab-cli/src/app.rs` — `check_responses` ToolProgress/ToolDone |
| Tool line formatting | `edgecrab/crates/edgecrab-cli/src/tool_display.rs` |
| Gateway throttled status | `edgecrab/crates/edgecrab-gateway/src/event_processor.rs` |
| Grades | `edgecrab/specs/002-terminal-ux-ui/005-honest-assessment.md` |

---

## Scenario replay (from [006-stuck-scenarios-playbook.md](006-stuck-scenarios-playbook.md))

| Scenario | Hermes user experience | EdgeCrab user experience |
|----------|------------------------|--------------------------|
| **S1 Long cargo build** | Spinner + elapsed; **no compiler lines** until done | Placeholder updates with **last 3 compiler lines** |
| **S2 Verbose off** | Silent transcript; status bar only | **`⏳` + tail** in system line |
| **S3 Background server** | Notification poller / gateway watcher; TUI `process.list` for manual tail | In-scrollback **`📟` monitor** line |
| **S4 wait_for_process** | Silent poll + activity touch | **2s tail heartbeat** |
| **S7 Compression** | No user-facing start/done event | **ActivityNotice** |
| **S8 Approval** | Overlay + activity touch | Overlay + **ActivityNotice** |
| **S12 Gateway Telegram** | Typing + **start-only** progress bubble | Typing + **throttled tail** status |

---

## Parity gaps to close (EdgeCrab ← Hermes)

| Gap | Status | Notes |
|-----|--------|-------|
| Background tail length in UI | **Closed** | `/tail` overlay + 4KB body |
| `tool.generating` | **Closed** | `StreamEvent::ToolGenerating` + shelf phase |
| Long-run charms | **Closed** | 10s interval, warn tone, 4/turn cap |
| Ink thinking shelf / `/details` | **Closed (semantics)** | `activity_shelf` + `shelf_details`; persisted in `config.yaml` |
| Sub-agent tool churn in shelf | **Closed** | `tool_count` + current tool on delegate rows |
| Gateway bg running updates | **Closed** | `gateway.bg_tail_chars: 500` default in `event_processor` |
| Ink accordion polish | **Closed (static)** | Heat + sparklines + chevrons; no animated expand |

## Regressions to avoid (EdgeCrab → Hermes)

Do **not** drop live tail to match Hermes lifecycle-only model — EdgeCrab’s differentiator for terminal-heavy users is **`ToolProgress` during foreground execution**.

---

## Updated EdgeCrab self-grades (post-comparison)

| Dimension | EdgeCrab grade | vs Hermes |
|-----------|----------------|-----------|
| Tool lifecycle | A | ≈ equal |
| Tool mid-run detail | A− | **EdgeCrab ahead** |
| Terminal stdout mirroring | B− | **EdgeCrab ahead** (both far from emulator) |
| Background process visibility | A− | **EdgeCrab ahead** (`/tail` 4KB) |
| Gateway parity | B− | Mixed — EdgeCrab mid-run tail; Hermes bg push |
| UI chrome / shelf | **A** | **Parity** — heat + sparklines; Ink animation still smoother |
| `/details` + persistence | A | **EdgeCrab ahead** (same semantics, YAML persist) |

**Overall:** EdgeCrab **A / A** (visibility / liveness) vs Hermes **~B+ / B** — EdgeCrab trades Ink sparklines for **live tail + persisted shelf disclosure**.

---

## Cross-references

- EdgeCrab assessment → [005-honest-assessment.md](005-honest-assessment.md)
- Stuck scenarios → [006-stuck-scenarios-playbook.md](006-stuck-scenarios-playbook.md)
- Stream contract → [004-stream-event-contract.md](004-stream-event-contract.md)
- Roadmap → [007-implementation-roadmap.md](007-implementation-roadmap.md)
