# 011 — Computer Use — Implementation Plan

**Branch:** `feat/computer-use`  
**Source of truth:** Hermes `tools/computer_use/` + `cua-driver` MCP backend

## Architecture

Hermes v0.14 uses **`cua-driver mcp`** (stdio MCP) — not in-process CGEvent.
EdgeCrab mirrors that: Rust `ComputerUseBackend` trait + `CuaDriverBackend`
(MCP client) + `NoopBackend` (CI/tests).

```
edgecrab-tools/src/tools/computer_use/
├── mod.rs           ToolHandler + registration
├── backend.rs       trait + CaptureResult / ActionResult / UIElement
├── schema.rs        OpenAI function schema (matches Hermes enum/actions)
├── safety.rs        blocked key combos, type patterns, approval gate
├── dispatch.rs      action routing (capture, click, type, …)
├── response.rs      multimodal envelope, screenshot cache on disk
├── mcp.rs           dedicated stdio MCP session for cua-driver
├── cua_backend.rs   Hermes CuaDriverBackend port
├── noop.rs          test/CI stub
├── permissions.rs   macOS availability probe
└── tests.rs         ≥15 unit tests (noop backend)
```

## Wiring

| Layer | Change |
|-------|--------|
| `config.rs` | `computer_use.enabled` (default **false**), `keep_last_n_screenshots: 3` |
| `config_ref.rs` | `computer_use_enabled`, backend cmd |
| `toolsets.rs` | `COMPUTER_USE` toolset (opt-in, not in core) |
| `conversation.rs` | parse `_multimodal` tool JSON → `Message` with `Content::Parts`; screenshot prune hook |
| `compression.rs` | strip image parts from old `computer_use` tool messages (> N captures) |
| `commands.rs` + catalog | `/computer status\|permissions` (CLI + gateway) |
| `vision_routing.rs` | aux vision routing for non-vision main models (#24015) |
| `approval_runtime` | destructive actions respect yolo + approval_tx |

## Multimodal strategy (Rust constraint)

`edgequake-llm` tool role = string content only. Mitigations (Hermes-aligned):

1. **SOM/AX text** — element index list always in tool text (works text-only).
2. **Vision routing** — implemented: `vision_routing.rs` + `aux_vision.rs` + shared `analyze_local_image`.
3. **Vision main model** — store PNG under `~/.edgecrab/cache/computer_use/`; attach
   `Content::Parts` on `Message::tool_result`; `build_chat_messages` promotes image
   parts to `ChatMessage.images` on a synthetic follow-up user line tagged
   `[computer_use capture]` within the same tool-turn batch (documented in proof).

## Phased OS support

| Phase | OS | Backend |
|-------|-----|---------|
| **1 (this PR)** | macOS | cua-driver MCP |
| 2 | Linux X11 | future native/xtest |
| 3 | Wayland | ashpd portal |
| 4 | Windows | SendInput |

## Tests

- Schema parity vs Hermes (actions, max_elements bounds)
- Noop dispatch for all actions
- Safety blocks destructive keys / shell patterns in `type`
- `max_elements` coercion (1..1000, default 100)
- Permission probe returns structured error off macOS
- Compression retains last 3 screenshot parts
