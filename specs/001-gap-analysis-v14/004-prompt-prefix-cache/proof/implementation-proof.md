# 004 — Prompt Prefix Cache — Implementation Proof

**Date:** 2026-05-24  
**Branch:** `feat/prompt-prefix-cache` (edgecrab)  
**edgequake-llm:** [PR #76](https://github.com/raphaelmansuy/edgequake-llm/pull/76) → squash-merged; tags `v0.6.21` / `v0.6.22`

---

## What Was Broken

EdgeCrab already split the system prompt into `stable` / `dynamic` zones and set `cache_control` on `ChatMessage`s, but **edgequake-llm’s Anthropic provider flattened all system messages into a single string**, dropping `cache_control`. Cross-session and cross-turn prefix caching could not work regardless of prompt layout.

## What We Shipped

### edgequake-llm 0.6.22

| Item | Status |
|------|--------|
| `CacheControl.ttl` (`5m` / `1h`) | Done |
| `CachePromptConfig.cache_ttl` | Done |
| Anthropic `system` as text **or** block array with `cache_control` | Done |
| User messages with `cache_control` → block layout | Done |
| `anthropic-beta: prompt-caching-2024-07-31,extended-cache-ttl-2025-04-11` when TTL is `1h` | Done |
| Unit tests (`test_system_message_with_cache_control_emits_block_with_ttl`, etc.) | Done |

### edgecrab

| Item | Status |
|------|--------|
| `cache.prompt_prefix.enabled` / `ttl` in `AppConfig` | Done |
| `PromptBuilder::build_blocks()` stable/dynamic split (pre-existing, hardened) | Done |
| Date-only volatile stamp (`Conversation started: …`) — Hermes parity | Done |
| `cached_stable_prompt` + `build_chat_messages_blocks()` with 1h marker | Done |
| `#[non_exhaustive]` on `PromptBlocks` | Done |
| `stable_hash_invariant_under_session_and_cwd` | Done |
| `/cost` cache read + cache write lines | Done |
| `AGENTS.md` policy updated | Done |

---

## Tests Run

```bash
# edgequake-llm (on feat branch before merge)
cargo clippy --all-targets --all-features -- -D warnings   # pass
cargo test --lib                                          # 1239 pass (2 factory tests need clean env)

# edgecrab
cargo test -p edgecrab-core --lib                         # 453 pass
```

**Not run (credentials / provider):**

- Live Anthropic two-session `cache_read_input_tokens` ≥ 80% stable block (acceptance criterion — needs `ANTHROPIC_API_KEY` + identical SOUL/AGENTS).
- `cargo test -p edgecrab-core --test e2e_copilot -- --ignored` — Copilot path does not exercise Anthropic cache markers.

---

## Dependency Pin

Workspace `Cargo.toml` uses:

```toml
edgequake-llm = { git = "https://github.com/raphaelmansuy/edgequake-llm.git", tag = "v0.6.22", features = ["bedrock"] }
```

**Reason:** crates.io publish workflow for `v0.6.22` hit the security-audit job on first run; `v0.6.21` publish failed rustfmt gate. Tag `v0.6.22` is on GitHub; switch to `version = "0.6.22"` once `cargo search edgequake-llm` reports 0.6.22.

---

## Brutal Honest Assessment vs Nous Hermes Agent

| Dimension | Hermes | EdgeCrab (after this work) | Verdict |
|-----------|--------|----------------------------|---------|
| Stable/volatile split | `stable` / `context` / `volatile` | `stable` / `dynamic` (context files in dynamic) | **Parity** on cache-critical separation |
| Timestamp granularity | Date-only in volatile | Date-only in volatile | **Parity** |
| `cache_control` on wire | Native adapter + `apply_anthropic_cache_control` | edgequake-llm Anthropic provider (was missing; **fixed in 0.6.22**) | **Now parity** |
| 1h TTL config | `prompt_caching.cache_ttl` | `cache.prompt_prefix.ttl` | **Parity** (different YAML path) |
| OpenRouter / third-party Anthropic gateways | Policy matrix in `_anthropic_prompt_cache_policy` | Anthropic provider name only in `provider_supports_prompt_caching` | **Hermes ahead** — EdgeCrab does not cache via OpenRouter Claude yet |
| Per-turn cache telemetry in UI | Usage pricing surfaced | `/cost` shows cache read/write; session counters | **Rough parity** |
| Proof of 80% cache hit across sessions | Battle-tested in production | Architecture + unit tests only | **Hermes ahead** until live Anthropic verification |

**Rust-specific note:** Centralizing transport in `edgequake-llm` is the right SOLID boundary; the bug was entirely in the adapter layer, not EdgeCrab’s prompt builder.

**Overall:** After `edgequake-llm` 0.6.22, EdgeCrab **matches or exceeds** Hermes on the core mechanism (stable prefix + `cache_control` + 1h TTL). Remaining gaps are **provider routing breadth** (OpenRouter/native policy) and **production telemetry proof**, not the prompt-split design.

---

## Acceptance Criteria Checklist

| Criterion | Status |
|-----------|--------|
| Second session cache_read ≥ 80% stable (live) | **Not verified** — needs Anthropic E2E |
| SOUL.md change invalidates cache | **By design** (content hash); not live-tested |
| Non-Anthropic providers unchanged | **Pass** (gated in `prompt_cache_config_for`) |
| `/cost` columns | **Pass** (read + write) |
| `stable_hash` test | **Pass** |
| `cargo clippy` workspace | **Not re-run full workspace** after edgecrab edits |
| `#[non_exhaustive]` PromptBlocks | **Pass** |
| AGENTS.md | **Pass** |
