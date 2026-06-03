#!/usr/bin/env bash
# e2e-tests/run.sh — Local + CI driver for the Playwright E2E suite.
#
# Pipeline:
#   1. start the backend stack (docker compose up -d backend, postgres, ai)
#   2. wait for the health endpoints to come up
#   3. run the lightweight Node smoke (runner.mjs)
#   4. run the full Playwright spec set
#   5. drop a single report at e2e-report/index.html
#
# Usage:
#   ./e2e-tests/run.sh            # full pipeline
#   ./e2e-tests/run.sh --smoke    # only runner.mjs (no Playwright)
#   ./e2e-tests/run.sh --specs    # only Playwright (assumes services are up)
#   ./e2e-tests/run.sh --stop     # docker compose down afterwards
#
# Env knobs (all optional):
#   E2E_BASE_URL       default: http://localhost:8080
#   AI_SERVICE_URL     default: http://localhost:8001
#   COMPOSE_PROJECT    default: quant-trading
#   KEEP_SERVICES=1    do not stop the stack at the end (useful for debugging)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

MODE="${1:-all}"
KEEP_SERVICES="${KEEP_SERVICES:-0}"
BASE_URL="${E2E_BASE_URL:-http://localhost:8080}"
AI_URL="${AI_SERVICE_URL:-http://localhost:8001}"
COMPOSE_PROJECT="${COMPOSE_PROJECT:-quant-trading}"
HEALTH_TIMEOUT="${HEALTH_TIMEOUT:-120}"   # seconds
REPORT_DIR="$ROOT/e2e-report"
LOG_DIR="$ROOT/e2e-logs"
mkdir -p "$REPORT_DIR" "$LOG_DIR"

log()  { printf '\033[1;34m▶ %s\033[0m\n' "$*"; }
warn() { printf '\033[1;33m⚠ %s\033[0m\n' "$*" >&2; }
fail() { printf '\033[1;31m✗ %s\033[0m\n' "$*" >&2; exit 1; }

cleanup() {
  if [[ "$KEEP_SERVICES" != "1" && "$STARTED_SERVICES" == "1" ]]; then
    log "Stopping docker compose stack ($COMPOSE_PROJECT)…"
    docker compose -p "$COMPOSE_PROJECT" down --remove-orphans >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

STARTED_SERVICES=0

start_services() {
  log "Starting backend stack via docker compose…"
  docker compose -p "$COMPOSE_PROJECT" up -d backend postgres python-ai-service >/dev/null
  STARTED_SERVICES=1
}

wait_for_url() {
  local url="$1" name="$2" deadline=$((SECONDS + HEALTH_TIMEOUT))
  while (( SECONDS < deadline )); do
    if curl -fsS --max-time 2 "$url" >/dev/null 2>&1; then
      log "$name is up ($url)"
      return 0
    fi
    sleep 2
  done
  warn "$name did not become ready within ${HEALTH_TIMEOUT}s ($url)"
  return 1
}

run_smoke() {
  log "Running Node smoke runner.mjs (BASE=$BASE_URL AI=$AI_URL)…"
  if ! E2E_BASE_URL="$BASE_URL" AI_SERVICE_URL="$AI_URL" \
      node e2e-tests/runner.mjs 2>&1 | tee "$LOG_DIR/smoke.log"; then
    warn "Smoke runner reported failures — continuing to Playwright anyway."
    SMOKE_FAILED=1
  fi
}

run_specs() {
  log "Running Playwright spec set…"
  if ! E2E_BASE_URL="$BASE_URL" AI_SERVICE_URL="$AI_URL" \
      npx playwright test --reporter=list,html "$@" 2>&1 | tee "$LOG_DIR/playwright.log"; then
    PW_FAILED=1
  fi
}

PW_FAILED=0
SMOKE_FAILED=0

case "$MODE" in
  --smoke) run_smoke ;;
  --specs)  run_specs ;;
  --stop)
    docker compose -p "$COMPOSE_PROJECT" down --remove-orphans
    log "Stack stopped."
    exit 0
    ;;
  all|*)
    start_services
    wait_for_url "$BASE_URL/health"         "Rust backend" || warn "backend never came up — specs will likely fail"
    wait_for_url "$AI_URL/api/v1/health"   "AI service"   || warn "AI service never came up — AI specs will likely fail"
    run_smoke
    run_specs
    ;;
esac

log "Report: $REPORT_DIR/index.html"
if (( PW_FAILED || SMOKE_FAILED )); then
  fail "E2E pipeline finished with failures (smoke=$SMOKE_FAILED, playwright=$PW_FAILED)."
fi
log "All E2E checks passed."
