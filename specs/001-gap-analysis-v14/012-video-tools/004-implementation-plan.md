# 012 — Implementation Plan

## Architecture (ASCII)

```
   ┌──────────────────────────────────────────────────────────────────┐
   │             edgecrab-tools/src/tools/video/                     │
   │                                                                  │
   │   mod.rs                                                         │
   │   analyze.rs       (video_analyze tool — frame sampling)         │
   │   generate.rs      (video_generate tool — dispatch by backend)   │
   │   status.rs        (video_status, video_download tools)          │
   │   ffmpeg.rs        (sandboxed Command wrapper)                   │
   │   backend.rs       (VideoBackend trait)                          │
   │   backends/                                                      │
   │     veo.rs         (Google Veo)                                  │
   │     sora.rs        (OpenAI Sora)                                 │
   │     runway.rs      (Runway Gen-N)                                │
   │     pika.rs        (Pika)                                        │
   │     kling.rs       (Kling AI)                                    │
   │     hailuo.rs      (MiniMax Hailuo)                              │
   └──────────────────────────────────────────────────────────────────┘
                                  ▲
   ┌──────────────────────────────────────────────────────────────────┐
   │      edgecrab-core — multimodal piping (reuse from vision)      │
   └──────────────────────────────────────────────────────────────────┘
```

## File Map

| Action | Path |
|--------|------|
| **New tool** | `crates/edgecrab-tools/src/tools/video/analyze.rs` |
| **New tool** | `crates/edgecrab-tools/src/tools/video/generate.rs` |
| **New tool** | `crates/edgecrab-tools/src/tools/video/status.rs` (two ToolHandlers: status + download) |
| **Trait** | `crates/edgecrab-tools/src/tools/video/backend.rs` — `async fn submit() -> JobId; async fn status(JobId) -> JobStatus; async fn download(JobId) -> Bytes;` |
| **Backends** | `crates/edgecrab-tools/src/tools/video/backends/*.rs` — one per provider |
| **FFmpeg wrapper** | `crates/edgecrab-tools/src/tools/video/ffmpeg.rs` — runs `ffmpeg` with **fully argv-quoted** args; never `sh -c`; validates input path via `edgecrab-security::path_safety` and URL via `is_safe_url` |
| **Frame sampling** | helper functions: uniform sample of N frames; scene-change sample via ffmpeg `select='gt(scene,X)'` |
| **Audio transcription** | reuse `transcribe` tool internals via direct function call (DRY) |
| **Multimodal piping** | frames returned as `Vec<ImagePart>` consumed by next assistant turn (same code path as `vision`) |
| **Config** | `video.ffmpeg_path` (default `ffmpeg` from $PATH), `video.max_frames` (default 16), `video.backends.<name>.api_key` |
| **Slash command** | `/video status <job_id>` thin wrapper to the tool |
| **Tests** | tiny test video fixture in `tests/fixtures/video/test_2s.mp4`; mock backends for `video_generate` |

## Security — Critical

- `ffmpeg` invocation: **never** pass URL/path through a shell; always
  use `Command::arg`. Validate inputs:
  - Path: `edgecrab-security::path_safety::validate_path`.
  - URL: `edgecrab-security::ssrf::is_safe_url`.
- Output frames written to a per-call temp dir; auto-cleaned on tool
  return (use `tempfile::TempDir`).
- Set ffmpeg resource limits via cgroup/nice (Linux) or `nice` (macOS)
  to prevent runaway transcodes; default `-t 300` (max 5 min video).
- Reject videos > configurable size (default 500 MB).

## Cost Model

A 16-frame multimodal call ≈ 16 × ~1,500 tokens = 24,000 input tokens
on Anthropic. Default `max_frames: 16` is a balance; users override
via tool argument. Document expected cost prominently in tool schema
description.

## Polling vs. Blocking

`video_generate` defaults to **non-blocking**: returns `job_id`
immediately. This is critical for chat-style flows where blocking 90s
on a Sora call would be terrible UX. The agent then either polls via
`video_status` or schedules a `cron` job to notify the user when ready.

## DRY / SOLID Notes

- **DIP:** `VideoBackend` trait isolates per-provider quirks.
- **OCP:** new backend = new file in `backends/`. No changes elsewhere.
- **SRP:** sampling, transcription, generation each separate file.
- **DRY:** frame→ImagePart conversion reuses the `vision` plumbing.

## Cross-References

- [001-overview.md](001-overview.md) · [005-acceptance-criteria.md](005-acceptance-criteria.md)
