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
: "${CHARTS_INT_DATABASE_URL:?Set an explicit isolated QA database URL; runtime DATABASE_URL is never a fixture target}"
export DATABASE_URL="$CHARTS_INT_DATABASE_URL"


_fail() {
  echo "ERROR: $*" >&2
  exit 1
}

_require_cmd() {
  local cmd=$1
  local hint=$2
  command -v "$cmd" >/dev/null 2>&1 || _fail "$hint"
}

_check_isolated_qa() {
  local binary="${INDEXER_QA_BIN:-$REPO_ROOT/indexer/target/release/cl8y-dex-indexer}"
  [[ -x "$binary" ]] || _fail 'Build the indexer binary first; isolated QA identity must be verified before fixture writes'
  # Shared Rust policy checks mode, effective URL, authenticated role/marker and
  # pre-applied schema. No database creation/migration or mutation in this check.
  "$binary" seed-qa --check
}

_require_indexer() {
  if ! curl -sf "${VITE_INDEXER_URL%/}/health" >/dev/null 2>&1; then
    _fail "Indexer HTTP not reachable at ${VITE_INDEXER_URL}. Start the indexer first (make start-qa or make indexer-dev) and ensure it uses the explicitly prepared isolated QA database"
  fi
}

_seed_fixtures() {
  echo "[test-charts-integration] seeding charts integration fixtures (idempotent)"
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$REPO_ROOT/indexer/scripts/seed-charts-integration.sql"
}

# shellcheck source=scripts/lib/localterra-host-curl.sh
source "$REPO_ROOT/scripts/lib/localterra-host-curl.sh"

_lcd_curl() {
  local path="${1:?}"
  localterra_lcd_curl "${VITE_TERRA_LCD_URL}" "$path"
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

echo "[test-charts-integration] checking isolated QA target (credentials omitted)"
_check_isolated_qa
_require_cmd psql "psql not found. Install PostgreSQL client tools."
_seed_fixtures
_require_indexer
_run_vitest
