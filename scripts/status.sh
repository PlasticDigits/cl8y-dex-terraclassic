#!/usr/bin/env bash
# Service status: Docker, LocalTerra, Postgres, indexer (CL8Y DEX QA / dev).
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  RED='\033[0;31m'
  BLUE='\033[0;34m'
  NC='\033[0m'
else
  GREEN=''
  YELLOW=''
  RED=''
  BLUE=''
  NC=''
fi

set -a
if [ -f "$REPO_ROOT/.env" ]; then
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
fi
if [ -f "$REPO_ROOT/scripts/qa/qa-host.env" ]; then
  # shellcheck source=/dev/null
  source "$REPO_ROOT/scripts/qa/qa-host.env"
fi
set +a

if [ "${QA_SHARED_HOST:-}" = "1" ]; then
  export COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.yml:docker-compose.qa-shared-host.yml}"
fi

TERRA_RPC_URL="${TERRA_RPC_URL:-http://127.0.0.1:${DEX_TERRA_RPC_PORT:-26657}}"
API_PORT="${API_PORT:-3001}"
PIDFILE="${REPO_ROOT}/.indexer-qa.pid"

log_line() {
  local name="$1"
  local state="$2"
  local detail="${3:-}"
  local color="$RED"
  if [ "$state" = "ok" ]; then
    color="$GREEN"
  elif [ "$state" = "warn" ]; then
    color="$YELLOW"
  fi
  printf "  %-18s " "${name}:"
  echo -e "${color}${state}${NC} ${detail}"
}

echo ""
echo "========================================"
echo "    CL8Y DEX — service status"
echo "========================================"
echo ""

echo -e "${BLUE}Docker compose:${NC}"
if docker info >/dev/null 2>&1; then
  docker compose ps
else
  log_line "Docker" "down" "(docker not reachable)"
fi
echo ""

echo -e "${BLUE}Health:${NC}"

if curl -sf "${TERRA_RPC_URL}/status" >/dev/null 2>&1; then
  height=""
  height=$(curl -sf "${TERRA_RPC_URL}/status" | jq -r '.result.sync_info.latest_block_height // empty' 2>/dev/null || true)
  log_line "LocalTerra" "ok" "(${TERRA_RPC_URL} block ${height:-?})"
else
  log_line "LocalTerra" "down" "(${TERRA_RPC_URL})"
fi

if docker compose exec -T postgres pg_isready -U postgres >/dev/null 2>&1; then
  log_line "Postgres" "ok" "(compose service postgres)"
else
  log_line "Postgres" "down" ""
fi

if curl -sf "http://127.0.0.1:${API_PORT}/health" >/dev/null 2>&1; then
  log_line "Indexer" "ok" "(http://127.0.0.1:${API_PORT}/health)"
else
  log_line "Indexer" "down" "(http://127.0.0.1:${API_PORT}/health)"
fi

if [ -f "$PIDFILE" ]; then
  pid="$(cat "$PIDFILE" 2>/dev/null || true)"
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    log_line "Indexer pidfile" "ok" "(pid ${pid})"
  else
    log_line "Indexer pidfile" "warn" "(${PIDFILE} stale or empty)"
  fi
else
  log_line "Indexer pidfile" "warn" "(no ${PIDFILE#"$REPO_ROOT"/} — ok if not using start-qa)"
fi

echo ""
