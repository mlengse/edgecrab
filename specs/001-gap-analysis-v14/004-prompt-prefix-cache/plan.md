# 004 — Implementation Plan (executed)

## Phase 1 — edgequake-llm (v0.6.22)

- [x] `CacheControl::ttl` + `ephemeral_ttl("1h"|"5m")`
- [x] `CachePromptConfig::cache_ttl`
- [x] Anthropic provider: serialize `cache_control` on system blocks + user messages
- [x] Auto `anthropic-beta` header for 1h tier
- [x] PR [#76](https://github.com/raphaelmansuy/edgequake-llm/pull/76) squash-merged; tag `v0.6.22`

## Phase 2 — edgecrab-core

- [x] `AppConfig.cache.prompt_prefix` (`enabled`, `ttl`)
- [x] `AgentConfig.cache` projection
- [x] `build_chat_messages_blocks` + `stable_cache_control()` wire TTL
- [x] Date-only volatile timestamp (Hermes parity)
- [x] `#[non_exhaustive]` on `PromptBlocks`
- [x] `stable_hash_invariant_under_session_and_cwd` test
- [x] `/cost` shows cache read + cache write columns

## Phase 3 — Verification

- [x] `cargo test -p edgecrab-core --lib`
- [ ] Live Anthropic: two sessions ≤1h apart → `cache_read_input_tokens` ≥80% stable size (requires API key)
- [ ] `cargo test -p edgecrab-core --test e2e_copilot -- --ignored` with `copilot/gpt-5-mini`

## Dependency

`edgequake-llm` pinned to git tag `v0.6.22` until crates.io publish completes; then switch workspace to `version = "0.6.22"`.
