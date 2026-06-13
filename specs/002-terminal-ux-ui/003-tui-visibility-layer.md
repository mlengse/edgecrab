# 003 — TUI Visibility Layer

What the EdgeCrab TUI actually renders during agent + terminal work. Primary file: [app.rs](../../crates/edgecrab-cli/src/app.rs). Live shelf modules: [turn_activity.rs](../../crates/edgecrab-cli/src/turn_activity.rs), [activity_shelf.rs](../../crates/edgecrab-cli/src/activity_shelf.rs).

## Layout (activity shelf)

When `display.activity_shelf: true` (default) and the agent is processing:

```text
┌─ Transcript (history) ─────────────────────────────┐
├─ Activity shelf (0–4 lines, ephemeral) ───────────│  ← live only
├─ Status bar ──────────────────────────────────────│
└─ Prompt ──────────────────────────────────────────│
```

**Rule:** Shelf = L1 live state. Transcript = durable narrative (policy-gated). Status bar = L0 compass.

| Module | Role |
|--------|------|
| `turn_activity.rs` | Single source of truth: active tools, phase, bg procs, subagents, activity feed |
| `activity_shelf.rs` | Render-only; no duplicate state |
| `live_progress.rs` | 16ms shelf redraw coalesce |

### Shelf contents

- **Phase line:** awaiting / thinking / preparing tool / analyzing output / streaming
- **Active tools:** ≤3 in-flight rows; **removed on ToolDone** (history lives in transcript)
- **Activity feed:** rolling long-run charms + onboarding (max 2 lines)
- **Sub-agents:** up to 2 delegate rows with `[i/n]` goal
- **Background:** headline + last 2 tail lines + `/tail` hint
- **Verbose mode + shelf:** in-flight args shown under tool row in shelf; transcript verbose lines suppressed

Compact / Termux (`<60` cols): shelf collapses to 1 summary line.

### `/details` (Hermes parity)

Independent of transcript `/verbose`. Controls shelf sections only.

| Command | Effect |
|---------|--------|
| `/details status` | Show effective mode per section |
| `/details collapsed` | Global collapsed (all sections) |
| `/details thinking expanded` | Override one section |
| `/details activity reset` | Restore Hermes default for section |

**Defaults:** thinking + tools **expanded**, activity **hidden**, subagents follow global (`collapsed`).

When activity is hidden, **warn/error** notices still show (Hermes error backstop).

## DisplayState machine

Defined at [app.rs L2592–2680](../../crates/edgecrab-cli/src/app.rs):

| State | Trigger | Status bar | Output pane |
|-------|---------|------------|-------------|
| `AwaitingFirstToken` | Turn start, post-tool LLM wait | Spinner + kaomoji verb; urgency color ramp | Ghost line: “awaiting response…” |
| `Thinking` | Reasoning tokens (if shown) | Same as above | Ghost “thinking…” if reasoning hidden |
| `Streaming` | First `Token` when streaming on | `▶ ~Nw`, t/s, section heading | Live markdown append |
| `ToolExec` | `ToolExec` event | Tool verb + command preview + spinner; `^C=stop` after 10s | Running placeholder (policy-dependent) |
| `WaitingForClarify` | `Clarify` event | Amber label | Modal overlay |
| `WaitingForApproval` | `Approval` event | Orange label | Approval overlay |
| `SecretCapture` | `SecretRequest` event | Red label | Masked input overlay |
| `BgOp` | Background ops (model transfer, etc.) | Spinner + label | **No ghost line** |
| `Idle` | Turn complete | Outcome badge / goal flash | — |

**Notable absence:** no output-pane ghost during `ToolExec`. If `/verbose off`, the scroll area can look frozen except status bar motion.

## Turn lifecycle (user-visible)

### 1. User submits message

[process_input](../../crates/edgecrab-cli/src/app.rs) ~L12155:

- `is_processing = true`
- `display_state = AwaitingFirstToken`
- Spawns `chat_streaming`

### 2. LLM cold start

Status bar: [render_status_bar L24006–24018](../../crates/edgecrab-cli/src/app.rs) — `format_waiting_first_token_status` with elapsed seconds.

Output: ghost waiting line ([render_output](../../crates/edgecrab-cli/src/app.rs) ~L23629–23655).

**FP46 urgency ramp:** `wait_urgency_color(elapsed_secs)` — amber → orange → red for long waits.

### 3. Token streaming

[check_responses](../../crates/edgecrab-cli/src/app.rs) L13041–13117 handles `AgentResponse::Token`:

| Flag | Source | Behavior |
|------|--------|----------|
| `streaming_enabled` | `model.streaming && display.streaming` | API streaming + status metrics |
| `live_token_display_enabled` | `streaming_enabled && !BasicCompat` | Incremental output pane |

If live display off: tokens buffer in `buffered_assistant_output` until flush at tool boundary or `Done` ([flush_buffered_assistant_output L5671–5705](../../crates/edgecrab-cli/src/app.rs)).

**Stuck feeling:** BasicCompat / streaming-off users see spinner only until flush — no partial answer.

### 4. Tool start (`ToolExec`)

[check_responses L13177–13257](../../crates/edgecrab-cli/src/app.rs):

Critical behaviors:

1. **`flush_buffered_assistant_output()`** — commits pre-tool text
2. **`streaming_line = None`** — prevents post-tool tokens merging with pre-tool text
3. **`in_flight_tool_count++`** — parallel tool tracking
4. **`display_state = ToolExec`**
5. If `should_render_tool_call`: push amber placeholder via `build_tool_running_line_width`

Comment at L13219–13224 explicitly documents why placeholder exists — long tools otherwise freeze the output area.

Placeholder format (from tool_display.rs):

```text
┊ 💻 terminal  $ cargo build --release  ···
```

After 3s elapsed: placeholder and status bar show seconds ([tick_spinner L14583–14630](../../crates/edgecrab-cli/src/app.rs)).

### 5. Tool progress (`ToolProgress`)

[check_responses L13259–13323](../../crates/edgecrab-cli/src/app.rs):

- Updates status bar `detail`
- Upgrades placeholder in-place with progress message
- Fallback: plain system line if no placeholder

**Reality:** almost never fires for terminal/web/browser — see [002-terminal-and-process-tools.md](002-terminal-and-process-tools.md).

### 6. Tool done (`ToolDone`)

[check_responses L13325–13424](../../crates/edgecrab-cli/src/app.rs):

- Upgrades placeholder via `build_tool_done_line_width`
- Terminal: `format_terminal_result` → `✓ 0  Compiling edgecrab v…`
- `ToolProgressMode::Verbose`: extra detail lines
- File-edit tools: optional diff lines
- When `in_flight_tool_count` hits 0 → `AwaitingFirstToken` for next LLM call

### 7. Turn end (`Done`)

[check_responses L13609–13656](../../crates/edgecrab-cli/src/app.rs):

- `clear_active_request_state()`
- TTFB hint if ≥1s
- Token/cost auto-update
- Voice TTS if enabled

## Tool transcript policy (`/verbose`)

When **activity shelf is enabled** (default):

| `ToolProgressMode` | In-flight (L1) | On ToolDone (L2) |
|--------------------|----------------|------------------|
| `Off` | Shelf only | Transcript quiet |
| `New` / `All` | Shelf only | Done line in transcript |
| `Verbose` | Shelf + args row | Done line only (no extra verbose lines in transcript) |

When **shelf disabled** (legacy): in-flight placeholders follow `should_render_tool_call` as before.

[should_render_in_flight_tool_in_transcript](../../crates/edgecrab-cli/src/app.rs): returns `false` when shelf enabled.

Default: **`ToolProgressMode::Verbose`** ([config.rs](../../crates/edgecrab-core/src/config.rs)).

## Status bar composition

[render_status_bar L23975+](../../crates/edgecrab-cli/src/app.rs):

| Element | Purpose |
|---------|---------|
| EC badge + version | Brand anchor |
| Spinner / state text | Primary activity indicator |
| Model name | Current model |
| Token count + context gauge | Context pressure |
| Cost | Session spend |
| `⛵ N pending` / `⛵ applied` | Steering (3–4s flash) |
| Shadow judge badge | Safety intervention |
| `BG N` | Background task count |
| Active subagents chip | Delegation visibility |
| Turn count, scroll hints | Navigation |

During `ToolExec` ([L24068–24124](../../crates/edgecrab-cli/src/app.rs)): tool verb, icon, arg preview, elapsed, `^C=stop` after 10s.

Compact variant for narrow/slow terminals: `render_compact_status_bar` (L23976–23978).

## Spinner / redraw loop

[event_loop L31658+](../../crates/edgecrab-cli/src/app.rs):

1. `check_responses()` (non-blocking)
2. `tick_spinner()` every 80ms (250ms reduced-motion)
3. Throttled `terminal.draw()` via `needs_redraw`
4. Input poll (~16ms while processing)
5. `check_responses()` again

Reduced-motion: `animate_status_indicators = false` → static indicators + periodic heartbeat redraw (~L31768–31774).

## Sub-agents & background work

### Foreground delegation (`delegate_task`)

Same-turn sub-agent events ([check_responses L13426–13597](../../crates/edgecrab-cli/src/app.rs)):

- One running placeholder per sub-agent
- `SubAgentReasoning` / `SubAgentToolExec` update in-place
- Individual sub-agent tool calls **do not** become permanent transcript lines

### `/background` command

Isolated agent ([app.rs ~L18357–18448](../../crates/edgecrab-cli/src/app.rs)):

- Progress as **`OutputRole::System`** lines — can accumulate many
- Not in-place; unlike foreground tool placeholders
- Interactive tools (approval/clarify/secret) auto-denied in background

## Terminal-specific TUI behavior summary

| Moment | User sees |
|--------|-----------|
| `terminal` invoked | `$ <command>` in status bar + placeholder |
| 0–120s+ execution | Spinner + `···` → elapsed after 3s |
| Command completes | `✓/✗ exit_code  first-line` in placeholder |
| Full build log | **Not in transcript** unless agent quotes it in prose or user uses verbose extra lines |
| Background server | Nothing until agent polls + narrates |

## UX gaps (TUI-specific)

1. **No live terminal pane** — fundamental; use shelf tail + `/tail` for bg procs.
2. **Ink-level craft** — Hermes accordion/sub-agent tree still richer visually.
3. **`/details` per-section toggle** — not implemented (global verbose + shelf only).
4. **`SteerPending` dropped in bridge** — TUI uses optimistic local counter only.
5. **Background progress spam** — `/background` still appends system lines.
6. **Sub-agent tool churn** — shelf shows delegate goal, not per-tool tree (Phase 2).
7. **Hook events invisible** — lifecycle hooks don't appear in transcript.

## Cross-references

- Stream events → [004-stream-event-contract.md](004-stream-event-contract.md)
- Stuck scenarios → [006-stuck-scenarios-playbook.md](006-stuck-scenarios-playbook.md)
- Prior UX spec → [specs/05-improve-ux-tui.md](../05-improve-ux-tui.md)
- Code index → [008-cross-ref-index.md](008-cross-ref-index.md)
