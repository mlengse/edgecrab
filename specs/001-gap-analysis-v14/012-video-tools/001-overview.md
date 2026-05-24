# 012 — Video Analysis + Pluggable Video Generation

**Tier:** A | **Impact:** 4 | **Value-per-Effort:** 3 | **Risk:** 3
**Primitive moved:** Reach (qualitatively new modality)

## Why It Matters (First Principles)

Video is the modality with the highest information density per second of
human attention. Two distinct capabilities matter for agents:

1. **`video_analyze`** — given a video file or URL, sample frames at an
   adaptive cadence and feed them to a multimodal LLM. Use cases:
   "summarise this tutorial", "extract timestamps where the speaker
   talks about X", "find the frame where the bug appears", "transcribe
   slides from this conference recording".

2. **`video_generate`** — provider-pluggable interface to text-to-video
   backends: Veo (Google), Sora (OpenAI), Runway, Pika, Kling, Hailuo.
   The interface is identical; the backend is configurable.

Hermes v0.14 added both. EdgeCrab has neither.

## The Gap

EdgeCrab has `vision` (single image) and `transcribe` (audio). It has no
video analysis primitive and no video generation primitive. As model
providers ship video capabilities at an accelerating rate, EdgeCrab is
falling behind on modality coverage.

## What EdgeCrab Gets Wrong Today

If a user pastes a YouTube link asking "summarise this", the agent has
no tool. It might `web_extract` the page metadata, but it cannot see
the actual content. This is a *trust* failure: the user expects the
agent to handle video the same way it handles images and audio.

## Cross-References

- [002-hermes-reference.md](002-hermes-reference.md)
- [003-edgecrab-current-state.md](003-edgecrab-current-state.md)
- [004-implementation-plan.md](004-implementation-plan.md)
- [005-acceptance-criteria.md](005-acceptance-criteria.md)
