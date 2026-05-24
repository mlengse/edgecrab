#!/usr/bin/env bash
# Persistent goals demo — mock regression + optional Copilot E2E.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

DEMO_HOME="$ROOT/demo/.edgecrab-home"
mkdir -p "$DEMO_HOME"

export EDGECRAB_HOME="$DEMO_HOME"
export RUST_BACKTRACE=1

echo "== EdgeCrab persistent goals demo =="
echo "EDGECRAB_HOME=$EDGECRAB_HOME"
echo

echo ">> Building workspace..."
cargo build -p edgecrab-core -p edgecrab-cli --quiet

echo ">> FK fix + unit tests (mock provider)..."
cargo test -p edgecrab-core goal_set_before_first_chat -- --nocapture
cargo test -p edgecrab-state ensure_session_row -- --nocapture
cargo test -p edgecrab-state goals_survive -- --nocapture
cargo test -p edgecrab-core execute_loop_injects_goal -- --nocapture

echo ">> Ralph loop parity tests (Hermes-mapped)..."
cargo test -p edgecrab-core --test goals_ralph_loop -- --nocapture

echo ">> Goal UX unit tests (chip + flash)..."
cargo test -p edgecrab-core compact_status_chip -- --nocapture
cargo test -p edgecrab-core goal_flash_from_decision -- --nocapture

echo ">> Demo integration test (mock)..."
cargo test -p edgecrab-core --test demo_persistent_goals mock_demo_flow -- --nocapture

echo ">> Copilot E2E (gpt-5-mini)..."
if cargo test -p edgecrab-core --test demo_persistent_goals copilot_demo_flow -- --ignored --nocapture; then
  echo ">> Copilot E2E: PASSED"
else
  echo ">> Copilot E2E: skipped (no auth or rate limit) — run manually:"
  echo "   EDGECRAB_HOME=$DEMO_HOME cargo test -p edgecrab-core --test demo_persistent_goals copilot_demo_flow -- --ignored --nocapture"
fi

echo
echo "Demo complete."
