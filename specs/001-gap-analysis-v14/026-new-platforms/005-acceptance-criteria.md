# 026 — Acceptance Criteria

## Per Platform

- [ ] **LINE** — webhook receives message; agent reply delivered via
      Push API; HMAC signature verified; invalid signature → 401.
- [ ] **SimpleX** — `simplex-chat` subprocess spawned and managed;
      WS messages round-trip; subprocess killed on gateway shutdown.
- [ ] **Google Chat** — webhook authenticated via Google's JWT;
      replies use Card V2 when applicable; plain text fallback when
      not.
- [ ] **MS Teams** — Bot Framework activity flow round-trip; JWT
      validated against Microsoft's public keys; Adaptive Card
      replies; conversation reference persisted for proactive push.

## Shared

- [ ] Each adapter behind a Cargo feature; default build does NOT
      include all four (opt-in to control binary size).
- [ ] `MEDIA://` protocol works on all four (image/voice/file as
      platform-native upload).
- [ ] `CLARIFY://` (folder 015) works on LINE (Quick Reply), Google
      Chat (Card buttons), Teams (Adaptive Card actions); SimpleX
      falls back to text.

## Code Quality

- [ ] `cargo clippy --workspace -- -D warnings`.
- [ ] Each adapter has wiremock integration test for send + receive.
- [ ] No leaked subprocesses after `cargo test` (SimpleX).

## Documentation

- [ ] `AGENTS.md` gateway table extended with all four platforms +
      env vars + Cargo feature names.

## Cross-References

- [001-overview.md](001-overview.md) · [004-implementation-plan.md](004-implementation-plan.md)
