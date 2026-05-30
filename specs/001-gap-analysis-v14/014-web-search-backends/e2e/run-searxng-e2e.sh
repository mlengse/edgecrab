#!/usr/bin/env bash
# Start Docker SearXNG and run real web_search E2E tests.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${ROOT}/../../../.." && pwd)"
COMPOSE_FILE="${ROOT}/docker-compose.searxng.yml"
SEARXNG_URL="${SEARXNG_URL:-http://127.0.0.1:8888}"
MAX_WAIT="${SEARXNG_E2E_WAIT_SECS:-90}"

cd "${ROOT}"

if ! command -v docker >/dev/null 2>&1; then
  echo "error: docker is required for SearXNG E2E" >&2
  exit 1
fi

echo "==> Starting SearXNG (docker compose)..."
docker compose -f "${COMPOSE_FILE}" up -d --wait --wait-timeout "${MAX_WAIT}" 2>/dev/null \
  || docker compose -f "${COMPOSE_FILE}" up -d

echo "==> Waiting for SearXNG JSON API at ${SEARXNG_URL} (non-empty results)..."
deadline=$((SECONDS + MAX_WAIT))
until curl -sf "${SEARXNG_URL%/}/search?q=rust&format=json" \
  | python3 -c "import sys,json; d=json.load(sys.stdin); exit(0 if d.get('results') else 1)" 2>/dev/null; do
  if (( SECONDS >= deadline )); then
    echo "error: SearXNG did not become ready within ${MAX_WAIT}s" >&2
    docker compose -f "${COMPOSE_FILE}" logs --tail 50 searxng || true
    exit 1
  fi
  sleep 2
done

export SEARXNG_URL
export EDGECRAB_E2E_SSRF_ALLOW_LOCALHOST=1
export EDGECRAB_SEARXNG_DOCKER_E2E=1

echo "==> Running SearXNG E2E tests..."
cd "${REPO_ROOT}"
cargo test -p edgecrab-tools --test web_search_e2e \
  e2e_searxng_docker \
  -- --include-ignored --test-threads=1 --nocapture

echo "==> SearXNG E2E passed."

if [[ "${SEARXNG_E2E_LEAVE_RUNNING:-0}" != "1" ]]; then
  echo "==> Stopping SearXNG container..."
  docker compose -f "${COMPOSE_FILE}" down
fi
