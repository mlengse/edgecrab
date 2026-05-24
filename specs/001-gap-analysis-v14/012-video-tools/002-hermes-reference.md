# 012 — Hermes Reference

| Concern | Hermes file |
|---------|-------------|
| `video_analyze` tool | `hermes-agent/tools/video_analyze.py` |
| Frame sampling | `ffmpeg` subprocess: select frames at uniform OR scene-change cadence |
| Caption generation | passes sampled frames + optional audio transcript to active multimodal LLM via `ctx.llm` |
| `video_generate` tool | `hermes-agent/tools/video_generate.py` |
| Backend trait | `VideoBackend` ABC with `Veo`, `Sora`, `Runway`, `Pika`, `Kling`, `Hailuo` implementations |
| Async polling | video generation is long-running (30s–10min); tool returns a `job_id` then polls until ready, or returns immediately with `job_id` and a separate `video_status` tool checks |

## `video_analyze` Behaviour

```
video_analyze({ url|path, max_frames: 20, mode: "uniform"|"scene", with_audio: true })
  │
  ├─► ffmpeg -i input -vf "select='gt(scene,0.4)'" -vsync vfr frame_%04d.png
  ├─► whisper.cpp / openai-whisper for audio (if with_audio)
  ├─► call ctx.llm with [transcript_chunk, frame_1, frame_2, ...]
  └─► return { summary, timestamps, frames_used }
```

## `video_generate` Behaviour

```
video_generate({ prompt, backend: "veo"|"sora"|"runway"|..., duration_s, aspect, seed? })
  │
  ├─► resolve backend instance (config: api keys per provider)
  ├─► submit job, return job_id immediately (or block if `wait:true`)
  │
  ▼
video_status({ job_id }) -> { status, progress?, video_url? }
video_download({ job_id, path }) -> { saved_path }
```

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
- Same `VideoBackend` extensibility pattern as: [../009-pluggable-providers-plugins/](../009-pluggable-providers-plugins/)
