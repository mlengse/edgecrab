# 012 — EdgeCrab Current State

| Existing | File |
|----------|------|
| `vision` tool | `crates/edgecrab-tools/src/tools/vision.rs` |
| `transcribe` tool | `crates/edgecrab-tools/src/tools/transcribe.rs` |
| `tts` tool | `crates/edgecrab-tools/src/tools/tts.rs` |

## What Is Missing

1. No `video_analyze` tool.
2. No frame-extraction infrastructure (no `ffmpeg` integration).
3. No `video_generate` tool.
4. No `VideoBackend` trait nor any video-provider integrations.
5. No `video_status` / `video_download` polling tools.

## Honest Assessment

`ffmpeg` is the only credible cross-platform video decoder; we will
shell out to it rather than link `libav` (license + build complexity).
The `Command` invocation must be hardened — `ffmpeg` args are easy to
misuse for command injection if the URL/path is not validated.

Video generation has six+ active providers, all incompatible. The
trait must be wide enough to capture polling, progress, multi-shot
output, and aspect/duration constraints without being a leaky
abstraction.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
