#!/usr/bin/env bash
# One-command local charts integration tests: Postgres ensure → migrate → seed → Vitest.
# Fixture pair: terra1paircontractabc (see frontend-dapp/src/test/chartsIntegrationConstants.ts).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

set -a
if [ -f "$REPO_ROOT/.env" ]; then
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
fi
# shellcheck source=scripts/lib/postgres-dev.env
source "$REPO_ROOT/scripts/lib/postgres-dev.env"
set +a

VITE_INDEXER_URL="${VITE_INDEXER_URL:-http://127.0.0.1:3001}"
# Use 127.0.0.1 (not localhost) so curl/Vitest avoid ::1 when LocalTerra binds IPv4 only — GitLab #131.
VITE_TERRA_LCD_URL="${VITE_TERRA_LCD_URL:-http://127.0.0.1:1317}"
VITE_TERRA_RPC_URL="${VITE_TERRA_RPC_URL:-http://127.0.0.1:26657}"
CHARTS_INT_DATABASE_URL="${CHARTS_INT_DATABASE_URL:-$DATABASE_URL}"
export DATABASE_URL="$CHARTS_INT_DATABASE_URL"
export PGPASSWORD="$POSTGRES_PASSWORD"

_charts_db_name() {
  local url="${1:?}"
  local name="${url##*/}"
  name="${name%%\?*}"
  echo "$name"
}

CHARTS_DB="$(_charts_db_name "$DATABASE_URL")"

_fail() {
  echo "ERROR: $*" >&2
  exit 1
}

_require_cmd() {
  local cmd=$1
  local hint=$2
  command -v "$cmd" >/dev/null 2>&1 || _fail "$hint"
}

_ensure_charts_db() {
  local exists
  exists="$(psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" -d postgres -tAc \
    "SELECT 1 FROM pg_database WHERE datname='${CHARTS_DB}'" 2>/dev/null || true)"
  if [ "$exists" = "1" ]; then
    echo "[test-charts-integration] database ${CHARTS_DB} already exists"
    return 0
  fi
  echo "[test-charts-integration] creating database ${CHARTS_DB} (owner ${POSTGRES_USER})..."
  psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" -d postgres -v ON_ERROR_STOP=1 \
    -c "CREATE DATABASE \"${CHARTS_DB}\" OWNER \"${POSTGRES_USER}\";"
}

_require_postgres() {
  _require_cmd psql "psql not found. Install PostgreSQL client tools."
  if ! pg_isready -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" >/dev/null 2>&1; then
    _fail "Postgres unreachable at ${POSTGRES_USER}@${POSTGRES_HOST}:${POSTGRES_PORT}. Start Postgres (e.g. docker compose up -d postgres) or see skills/AGENTS_LOCAL_POSTGRES_DEV.md"
  fi
  if ! psql -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" -d postgres -c '\q' 2>/dev/null; then
    _fail "Cannot connect to Postgres as ${POSTGRES_USER}@${POSTGRES_HOST}:${POSTGRES_PORT}. Check credentials in scripts/lib/postgres-dev.env or repo .env"
  fi
}

_require_indexer() {
  if ! curl -sf "${VITE_INDEXER_URL%/}/health" >/dev/null 2>&1; then
    _fail "Indexer HTTP not reachable at ${VITE_INDEXER_URL}. Start the indexer first (make start-qa or make indexer-dev) and ensure it uses DATABASE_URL=${DATABASE_URL}"
  fi
}

_run_migrations_docker() {
  local cid net db_url
  _require_cmd docker "docker not found. Install Docker or run migrations manually: cd indexer && sqlx migrate run"
  cid="$(docker compose -f "$REPO_ROOT/docker-compose.yml" ps -q postgres 2>/dev/null || true)"
  if [ -z "$cid" ]; then
    _fail "Postgres docker compose service is not running. Start it: docker compose up -d postgres"
  fi
  net="$(docker inspect -f '{{range $k,$v := .NetworkSettings.Networks}}{{$k}}{{end}}' "$cid")"
  db_url="postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/${CHARTS_DB}"
  echo "[test-charts-integration] running migrations via docker network (${db_url})"
  docker run --rm \
    --network "$net" \
    -v "$REPO_ROOT/indexer:/indexer" \
    -v "$(command -v sqlx):/usr/local/bin/sqlx:ro" \
    -w /indexer \
    -e DATABASE_URL="$db_url" \
    ubuntu:24.04 \
    bash -c 'apt-get update -qq && apt-get install -y -qq libssl3 ca-certificates >/dev/null && sqlx migrate run'
}

_run_migrations() {
  _require_cmd sqlx "sqlx-cli not found. Install: cargo install sqlx-cli --version 0.8.6 --no-default-features --features rustls,postgres --locked"
  echo "[test-charts-integration] running migrations against ${DATABASE_URL}"
  if (cd "$REPO_ROOT/indexer" && timeout 45 sqlx migrate run); then
    return 0
  fi
  echo "[test-charts-integration] WARN: host sqlx migrate failed or timed out; retrying via docker compose postgres network" >&2
  _run_migrations_docker
}

_seed_fixtures() {
  echo "[test-charts-integration] seeding charts integration fixtures (idempotent)"
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$REPO_ROOT/indexer/scripts/seed-charts-integration.sql"
}

# Host curl to published 127.0.0.1:1317 can hang on some Linux setups; fall back to compose exec.
_lcd_curl() {
  local path="${1:?}"
  local url="${VITE_TERRA_LCD_URL%/}${path}"
  local out
  out="$(curl -sf --connect-timeout 2 --max-time 8 "$url" 2>/dev/null || true)"
  if [ -n "$out" ]; then
    echo "$out"
    return 0
  fi
  local lt_cid
  lt_cid="$(docker compose -f "$REPO_ROOT/docker-compose.yml" ps -q localterra 2>/dev/null || true)"
  if [ -z "$lt_cid" ]; then
    return 1
  fi
  docker compose -f "$REPO_ROOT/docker-compose.yml" exec -T localterra \
    curl -sf --connect-timeout 2 --max-time 8 "http://127.0.0.1:1317${path}" 2>/dev/null
}

_load_factory_address() {
  if [ -n "${VITE_FACTORY_ADDRESS:-}" ]; then
    echo "$VITE_FACTORY_ADDRESS"
    return 0
  fi
  if [ -f "$REPO_ROOT/frontend-dapp/.env.local" ]; then
    grep -m1 '^VITE_FACTORY_ADDRESS=' "$REPO_ROOT/frontend-dapp/.env.local" | cut -d= -f2- | tr -d '\r"'"'"
    return 0
  fi
  if [ -f "$REPO_ROOT/indexer/.env" ]; then
    grep -m1 '^FACTORY_ADDRESS=' "$REPO_ROOT/indexer/.env" | cut -d= -f2- | tr -d '\r"'"'"
    return 0
  fi
  echo ""
}

_export_limit_order_integration_env() {
  local factory lcd b64 resp pair lp t0 t1
  factory="$(_load_factory_address)"
  lcd="${VITE_TERRA_LCD_URL%/}"
  if [ -z "$factory" ]; then
    echo "[test-charts-integration] WARN: factory address unknown; limit-order tests (#166) use fallbacks in limitOrderIntegrationConstants.ts" >&2
    return 0
  fi
  if ! _lcd_curl "/cosmos/base/tendermint/v1beta1/node_info" >/dev/null 2>&1; then
    echo "[test-charts-integration] WARN: LCD unreachable at ${lcd}; limit-order tests (#166) need LocalTerra (make deploy-local)" >&2
    return 0
  fi
  _require_cmd jq "jq not found. Install jq to resolve the EMBER/CORAL pair for limit-order integration tests (#166)."
  b64=$(printf '%s' '{"pairs":{"limit":1}}' | base64 -w0 2>/dev/null || printf '%s' '{"pairs":{"limit":1}}' | base64 | tr -d '\n')
  resp="$(_lcd_curl "/cosmwasm/wasm/v1/contract/${factory}/smart/${b64}" 2>/dev/null || true)"
  pair="$(echo "$resp" | jq -r '.data.pairs[0].contract_addr // empty' 2>/dev/null || true)"
  lp="$(echo "$resp" | jq -r '.data.pairs[0].liquidity_token // empty' 2>/dev/null || true)"
  t0="$(echo "$resp" | jq -r '.data.pairs[0].asset_infos[0].token.contract_addr // empty' 2>/dev/null || true)"
  t1="$(echo "$resp" | jq -r '.data.pairs[0].asset_infos[1].token.contract_addr // empty' 2>/dev/null || true)"
  if [ -z "$pair" ] || [ -z "$t0" ] || [ -z "$t1" ]; then
    echo "[test-charts-integration] WARN: could not read first factory pair from ${lcd}; limit-order tests (#166) may fail" >&2
    return 0
  fi
  export VITE_LIMIT_ORDER_INTEGRATION_PAIR_ADDRESS="$pair"
  export VITE_LIMIT_ORDER_INTEGRATION_LP_TOKEN="$lp"
  export VITE_LIMIT_ORDER_INTEGRATION_TOKEN0="$t0"
  export VITE_LIMIT_ORDER_INTEGRATION_TOKEN1="$t1"
  echo "[test-charts-integration] limit-order fixture pair ${pair} (from factory ${factory})"
}

_run_vitest() {
  _export_limit_order_integration_env
  echo "[test-charts-integration] running frontend integration tests (VITE_INDEXER_URL=${VITE_INDEXER_URL})"
  bash "$REPO_ROOT/scripts/with-node.sh" --cwd frontend-dapp -- \
    env \
      VITE_INDEXER_URL="$VITE_INDEXER_URL" \
      VITE_TERRA_LCD_URL="$VITE_TERRA_LCD_URL" \
      VITE_TERRA_RPC_URL="$VITE_TERRA_RPC_URL" \
      VITE_LIMIT_ORDER_INTEGRATION_PAIR_ADDRESS="${VITE_LIMIT_ORDER_INTEGRATION_PAIR_ADDRESS:-}" \
      VITE_LIMIT_ORDER_INTEGRATION_LP_TOKEN="${VITE_LIMIT_ORDER_INTEGRATION_LP_TOKEN:-}" \
      VITE_LIMIT_ORDER_INTEGRATION_TOKEN0="${VITE_LIMIT_ORDER_INTEGRATION_TOKEN0:-}" \
      VITE_LIMIT_ORDER_INTEGRATION_TOKEN1="${VITE_LIMIT_ORDER_INTEGRATION_TOKEN1:-}" \
      npm run test:integration
}

echo "[test-charts-integration] DATABASE_URL=${DATABASE_URL}"
_require_postgres
_ensure_charts_db
_run_migrations
_seed_fixtures
_require_indexer
_run_vitest
