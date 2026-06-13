# 007 — Implementation Roadmap

Prioritized changes to reach **“never feel stuck”** for terminal-heavy workflows. Ordered by leverage × feasibility.

Each item lists **touch points** with code anchors.

---

## Implementation status (branch `feat/terminal-ux-live-progress`)

| Item | Status | Notes |
|------|--------|-------|
| **P0** Live terminal tail | **Done** | `tool_progress_tail.rs`, local + PTY backends, `terminal.rs` |
| **P0b** Parallel tool progress | **Done** | `turn_tool_progress_tx` cloned into parallel `DispatchContext` |
| **P1** ProcessTable → TUI | **Done** | `BackgroundProcessTail` + `BackgroundProcessFinished`; watch match → `ActivityNotice` |
| **P1b** wait_for_process heartbeat | **Done** | 2s `emit_progress` with tail snapshot |
| **P2** Compression lifecycle | **Done** | `ActivityNotice` start/done/circuit-breaker via shared formatters |
| **P2b** SteerPending from core | **Done** | `Agent::send_steering()` + TUI forward |
| **P3** Verbose-off indicator | **Done** | Dim `⏳` line + in-place tail on `ToolProgress`; removed on `ToolDone` |
| **P3b** Terminal expand (Ctrl+Shift+T) | **Done** | `OutputLine.expandable_body` for terminal / execute_code |
| **P4** ToolOutput variant | **Deferred** | ToolProgress sufficient for now |
| **P4b** SDK export | **Done** | Python + Node: `tool_progress`, `activity_notice`, bg process, steer |
| Remote backends (Docker/SSH/Modal) | **Done** | Docker streams live; batch backends: start milestone + tail on complete |
| Web tools (search/extract/crawl) | **Done** | `ToolProgress` milestones via `progress_fn_from_context` + chain fallback |
| Browser CDP tools | **Done** | All interactive tools emit milestones; vision includes analyze step |
| Local `execute_code` subprocess | **Done** | Streaming capture → `ToolProgressTail` |
| Hermes comparison doc | **Done** | [009-hermes-comparison.md](009-hermes-comparison.md) |

---

## Post-P3b: Delightful TUI (Phase D)

P0–P3b delivered **liveness** (tail progress, bg monitor, verbose-off indicator). The next tranche is **polish** — see **[010-delightful-tui-plan.md](010-delightful-tui-plan.md)**:

| Phase | Focus | ETA |
|-------|-------|-----|
| **D1** | Activity shelf (live zone) | 2–3 weeks |
| **D2** | `ToolGenerating` + long-run hints | 1 week |
| **D3** | `/tail` panel + gateway bg push | 1–2 weeks |
| **D4** | Reasoning + sub-agent shelf | 1–2 weeks |
| **D5** | Coalesce, skins, SDK, re-grade | 1 week |

---

## P0 — Live terminal tail via existing ToolProgress (highest leverage)

**Problem:** [005-honest-assessment.md](005-honest-assessment.md) gap #1 and #2 — stdout batch-only; `emit_progress` unused.

**Approach:** Wire stdout/stderr line batches from execution backends to `ctx.emit_progress()`:

| File | Change |
|------|--------|
| [terminal.rs](../../crates/edgecrab-tools/src/tools/terminal.rs) | Pass `ctx` into backend execute; throttle progress (e.g. 200ms / last 3 lines) |
| [backends/local.rs](../../crates/edgecrab-tools/src/tools/backends/local.rs) | On each `read_line` before fence, `emit_progress` truncated tail |
| [local_pty.rs](../../crates/edgecrab-tools/src/local_pty.rs) | Same on chunk drain |
| [app.rs check_responses ToolProgress](../../crates/edgecrab-cli/src/app.rs) | Already handles in-place update — **no TUI change required** |
| [tool_display.rs](../../crates/edgecrab-cli/src/tool_display.rs) | Optional: terminal-specific progress formatting (strip ANSI before display) |

**Acceptance criteria:**

- During `cargo build`, user sees last 1–3 output lines updating in placeholder
- Rate-limited to ≤5 updates/sec (avoid TUI flood)
- Full output still goes to LLM tool result unchanged

**Effort:** Medium (2–4 days). **Risk:** Low — uses existing event path proven by MoA.

---

## P0b — Parallel tool progress channel

**Problem:** [conversation.rs L4371](../../crates/edgecrab-core/src/conversation.rs) drops `event_tx` for parallel tools.

**Approach:**

- Clone `make_tool_progress_tx(dctx.event_tx)` into parallel `DispatchContext`
- Keep `event_tx: None` for ToolExec/ToolDone duplication (already sent pre-spawn) OR emit ToolDone from parent join only (current pattern)

| File | Change |
|------|--------|
| [conversation.rs L4359–4386](../../crates/edgecrab-core/src/conversation.rs) | Add `tool_progress_tx: make_tool_progress_tx(...)` to parallel inner context |

**Effort:** Small (hours). **Risk:** Low.

---

## P1 — ProcessTable → TUI subscription

**Problem:** Background output invisible — [002-terminal-and-process-tools.md](002-terminal-and-process-tools.md).

**Approach A (minimal):** Wire `watch_notification_tx` in `build_tool_context`:

| File | Change |
|------|--------|
| [conversation.rs build_tool_context L390](../../crates/edgecrab-core/src/conversation.rs) | Create channel; spawn forwarder → `StreamEvent::ToolProgress` or new `ProcessOutput` variant |
| [app.rs forward_stream_event_to_tui](../../crates/edgecrab-cli/src/app.rs) | Handle new variant if added |

**Approach B (richer):** Optional `/tail <process_id>` TUI panel reading ProcessTable directly (bypasses agent).

**Effort:** Medium. **Risk:** Medium — need process_id → active tool_call_id mapping.

---

## P1b — `wait_for_process` heartbeat

**Problem:** [process.rs L947–988](../../crates/edgecrab-tools/src/tools/process.rs) blocks silently.

**Approach:** Every 2s inside loop, `ctx.emit_progress(format!("still running… last lines:\n{}", tail))`.

**Effort:** Small. **Depends on:** P0b if parallel (usually sequential).

---

## P2 — Compression lifecycle events

**Problem:** Silent 5–15s compress — [004-stream-event-contract.md](004-stream-event-contract.md).

**Approach:** Add variants or reuse `HookEvent`:

```rust
StreamEvent::Notice { kind: "compression_start" | "compression_done", detail }
```

| File | Change |
|------|--------|
| [conversation.rs L1467–1570](../../crates/edgecrab-core/src/conversation.rs) | Emit before/after `compress_with_llm` |
| [app.rs](../../crates/edgecrab-cli/src/app.rs) | Map to system line + BgOp spinner |

**Effort:** Small. **Risk:** Low.

---

## P2b — Wire SteerPending from core

**Problem:** Dead variant; optimistic TUI counter only.

| File | Change |
|------|--------|
| [steering.rs / conversation.rs](../../crates/edgecrab-core/src/steering.rs) | Emit `SteerPending { count }` on send |
| [app.rs L748–752](../../crates/edgecrab-cli/src/app.rs) | Forward to status bar counter sync |

**Effort:** Small. **Risk:** Low.

---

## P3 — Verbose-off minimal tool indicator

**Problem:** [006-stuck-scenarios-playbook.md](006-stuck-scenarios-playbook.md) S2.

**Approach:** When `ToolProgressMode::Off`, still push a **single dim system line** on ToolExec (not full placeholder), or always show status-bar-only mode hint on first long wait.

| File | Change |
|------|--------|
| [should_render_tool_call](../../crates/edgecrab-cli/src/app.rs) | Split “transcript detail” from “activity indicator” |

**Effort:** Small. **Risk:** Low — UX policy decision.

---

## P3b — Terminal output expand (progressive disclosure)

**Problem:** One-line done preview insufficient — [tool_display.rs L1823](../../crates/edgecrab-cli/src/tool_display.rs).

**Approach:** Ctrl+O / click to expand full tool result inline (per [05-improve-ux-tui.md](../05-improve-ux-tui.md) inspiration).

| File | Change |
|------|--------|
| [app.rs](../../crates/edgecrab-cli/src/app.rs) | Expand state on OutputLine for Tool role |
| [tool_display.rs](../../crates/edgecrab-cli/src/tool_display.rs) | Full terminal body renderer |

**Effort:** Medium. **Risk:** Low.

---

## P4 — New StreamEvent::ToolOutput (optional clean break)

If `ToolProgress` semantics become muddy (progress messages vs stdout chunks), add dedicated variant:

```rust
ToolOutput {
    tool_call_id: String,
    stream: OutputStream, // stdout | stderr
    chunk: String,
    truncated: bool,
}
```

Touches: agent.rs enum, conversation bridge, app.rs, event_processor.rs, SDK bindings.

**Effort:** Large. **Prefer P0 first** — validate with ToolProgress before new type.

---

## P4b — SDK full StreamEvent export

**Problem:** SDK subset — [004-stream-event-contract.md](004-stream-event-contract.md).

| File | Change |
|------|--------|
| [sdks/python/src/types.rs](../../sdks/python/src/types.rs) | Export RunFinished, ToolProgress, ContextPressure |
| Node SDK equivalent | Same |

**Effort:** Medium. **Risk:** Low.

---

## Suggested phasing

```text
Phase 1 (1 sprint):  P0 + P0b + P1b     → terminal feels alive
Phase 2 (1 sprint):  P1 + P2 + P2b      → background + compression clarity
Phase 3 (polish):    P3 + P3b + P4b     → power users + SDK
Phase 4 (optional):  P4                 → only if ToolProgress overloaded
```

## Non-goals (honest)

- **Full PTY mirror to user** — conflicts with agent-not-observing-screen design ([terminal.rs L156–157](../../crates/edgecrab-tools/src/tools/terminal.rs))
- **Replacing LLM tool results with streamed UI-only channel** — two sources of truth hazard
- **Gateway full log streaming** — platform message limits; tail summaries only

## Cross-references

- Assessment → [005-honest-assessment.md](005-honest-assessment.md)
- Scenarios → [006-stuck-scenarios-playbook.md](006-stuck-scenarios-playbook.md)
- Code index → [008-cross-ref-index.md](008-cross-ref-index.md)
