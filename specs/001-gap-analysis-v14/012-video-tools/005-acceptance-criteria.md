# 012 — Acceptance Criteria

## Functional

- [ ] `video_analyze({ path: "fixtures/test_2s.mp4", max_frames: 8 })`
      returns a summary; frames are visible in next assistant context.
- [ ] Scene-change sampling produces fewer frames for static content
      than uniform.
- [ ] Audio transcript included when `with_audio: true` and `ffmpeg`
      has an audio stream.
- [ ] `video_generate({ backend: "mock", prompt: "..." })` returns a
      `job_id` immediately (default non-blocking).
- [ ] `video_status({ job_id })` returns `queued` → `running` → `done`
      across polls.
- [ ] `video_download({ job_id, path })` writes the file; path validated
      via `path_safety`.
- [ ] At least one real backend (Sora or Runway) wired end-to-end and
      tested manually (CI uses mock).

## Security

- [ ] FFmpeg never invoked via shell; argv-only.
- [ ] URL/path validated through `is_safe_url` / `path_safety` before
      `ffmpeg` runs.
- [ ] Temp-dir cleanup on every code path (including panic via `Drop`).
- [ ] Oversize video rejected with clear error.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] ≥ 18 tests; mock `VideoBackend` covers all polling states.
- [ ] `video_*` tools registered in CORE_TOOLS list.

## Cost Transparency

- [ ] Tool description includes per-frame token cost estimate.
- [ ] `/cost` correctly accounts for frames in `video_analyze` calls.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
