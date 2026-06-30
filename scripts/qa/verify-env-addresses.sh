#!/usr/bin/env bash
# Cross-check frontend + indexer env contract addresses against each other and on-chain config.
# GitLab #442 — SEC-H04: detect env/chain address drift after manual staging/mainnet config.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

INDEXER_ENV="${VERIFY_ENV_INDEXER_FILE:-$REPO_ROOT/indexer/.env}"
FRONTEND_ENV="${VERIFY_ENV_FRONTEND_FILE:-$REPO_ROOT/frontend-dapp/.env.local}"

set -a
if [ -f "$REPO_ROOT/.env" ]; then
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
fi
# shellcheck source=/dev/null
source "$REPO_ROOT/scripts/qa/qa-host.env"
set +a

if [ "${QA_SHARED_HOST:-}" = "1" ]; then
  export COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.yml:docker-compose.qa-shared-host.yml}"
fi

LCD="${TERRA_LCD_URL:-http://127.0.0.1:${DEX_TERRA_LCD_PORT:-1317}}"
LCD="${LCD%/}"

# shellcheck source=scripts/lib/lcd-smart-query.sh
source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"

FAILURES=0
CHECKS=0

read_env_var() {
  local file="$1"
  local key="$2"
  sed -n "s/^${key}=//p" "$file" 2>/dev/null | head -1
}

assert_terra_addr() {
  local label="$1"
  local value="$2"
  CHECKS=$((CHECKS + 1))
  if [[ -n "$value" && "$value" =~ ^terra1 ]]; then
    echo "  [PASS] ${label}=${value}"
    return 0
  fi
  echo "  [FAIL] ${label} must be a non-empty terra1 address (got: ${value:-<empty>})" >&2
  FAILURES=$((FAILURES + 1))
  return 1
}

assert_eq() {
  local label="$1"
  local actual="$2"
  local expected="$3"
  CHECKS=$((CHECKS + 1))
  if [[ "$actual" == "$expected" ]]; then
    echo "  [PASS] ${label}=${actual}"
    return 0
  fi
  echo "  [FAIL] ${label}: expected ${expected}, got ${actual}" >&2
  FAILURES=$((FAILURES + 1))
  return 1
}

fail_query() {
  local label="$1"
  local detail="${2:-}"
  echo "  [FAIL] ${label}${detail:+ — ${detail}}" >&2
  FAILURES=$((FAILURES + 1))
}

if [ ! -f "$INDEXER_ENV" ]; then
  echo "ERROR: indexer env not found: ${INDEXER_ENV}" >&2
  exit 1
fi
if [ ! -f "$FRONTEND_ENV" ]; then
  echo "ERROR: frontend env not found: ${FRONTEND_ENV}" >&2
  exit 1
fi

IDX_FACTORY="$(read_env_var "$INDEXER_ENV" FACTORY_ADDRESS)"
IDX_ROUTER="$(read_env_var "$INDEXER_ENV" ROUTER_ADDRESS)"
IDX_FEE_DISCOUNT="$(read_env_var "$INDEXER_ENV" FEE_DISCOUNT_ADDRESS)"

VITE_FACTORY="$(read_env_var "$FRONTEND_ENV" VITE_FACTORY_ADDRESS)"
VITE_ROUTER="$(read_env_var "$FRONTEND_ENV" VITE_ROUTER_ADDRESS)"
VITE_FEE_DISCOUNT="$(read_env_var "$FRONTEND_ENV" VITE_FEE_DISCOUNT_ADDRESS)"

_RPC_URL="${TERRA_RPC_URL:-http://127.0.0.1:${DEX_TERRA_RPC_PORT:-26657}}"
if ! localterra_rpc_status_ok "$_RPC_URL"; then
  echo "ERROR: Terra RPC not reachable at ${_RPC_URL}; start chain and deploy first." >&2
  exit 1
fi

HEAD_SHA="$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "=== CL8Y DEX env address cross-check (SEC-H04 / GitLab #442) ==="
echo "timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "git_sha: ${HEAD_SHA}"
echo "lcd: ${LCD}"
echo "indexer_env: ${INDEXER_ENV}"
echo "frontend_env: ${FRONTEND_ENV}"
echo ""

# ── 1. Env file parity (indexer vs frontend) ───────────────────────────────
echo "[1/4] Env file parity (indexer vs frontend VITE_*)..."
assert_terra_addr "indexer FACTORY_ADDRESS" "$IDX_FACTORY" || true
assert_terra_addr "indexer ROUTER_ADDRESS" "$IDX_ROUTER" || true
assert_terra_addr "indexer FEE_DISCOUNT_ADDRESS" "$IDX_FEE_DISCOUNT" || true
assert_terra_addr "frontend VITE_FACTORY_ADDRESS" "$VITE_FACTORY" || true
assert_terra_addr "frontend VITE_ROUTER_ADDRESS" "$VITE_ROUTER" || true
assert_terra_addr "frontend VITE_FEE_DISCOUNT_ADDRESS" "$VITE_FEE_DISCOUNT" || true
assert_eq "FACTORY (indexer == frontend)" "$IDX_FACTORY" "$VITE_FACTORY" || true
assert_eq "ROUTER (indexer == frontend)" "$IDX_ROUTER" "$VITE_ROUTER" || true
assert_eq "FEE_DISCOUNT (indexer == frontend)" "$IDX_FEE_DISCOUNT" "$VITE_FEE_DISCOUNT" || true
echo ""

FACTORY="$IDX_FACTORY"
ROUTER="$IDX_ROUTER"
FEE_DISCOUNT="$IDX_FEE_DISCOUNT"

if [ -z "$FACTORY" ] || [ -z "$ROUTER" ] || [ -z "$FEE_DISCOUNT" ]; then
  echo "ERROR: FACTORY_ADDRESS, ROUTER_ADDRESS, and FEE_DISCOUNT_ADDRESS required in env files." >&2
  exit 1
fi

# ── 2. Factory contract at FACTORY_ADDRESS ─────────────────────────────────
echo "[2/4] Factory config at FACTORY_ADDRESS (query: config)..."
if ! lcd_smart_query_ok "$LCD" "$FACTORY" '{"config":{}}'; then
  fail_query "factory config at ${FACTORY}"
else
  FACTORY_CFG="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FACTORY" '{"config":{}}')")"
  GOV="$(echo "$FACTORY_CFG" | jq -r '.governance // empty')"
  TREASURY="$(echo "$FACTORY_CFG" | jq -r '.treasury // empty')"
  assert_terra_addr "factory.governance" "$GOV" || true
  assert_terra_addr "factory.treasury" "$TREASURY" || true
  echo "  factory contract at ${FACTORY} responds to config (on-chain factory address matches env)"
fi
echo ""

# ── 3. Router config.factory must equal env FACTORY_ADDRESS ────────────────
echo "[3/4] Router config at ROUTER_ADDRESS (query: config)..."
if ! lcd_smart_query_ok "$LCD" "$ROUTER" '{"config":{}}'; then
  fail_query "router config at ${ROUTER}"
else
  ROUTER_CFG="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$ROUTER" '{"config":{}}')")"
  CHAIN_FACTORY="$(echo "$ROUTER_CFG" | jq -r '.factory // empty')"
  assert_terra_addr "router.config.factory" "$CHAIN_FACTORY" || true
  assert_eq "router.config.factory == env FACTORY_ADDRESS" "$CHAIN_FACTORY" "$FACTORY" || true
fi
echo ""

# ── 4. Fee-discount config.governance non-empty ────────────────────────────
echo "[4/4] Fee-discount config at FEE_DISCOUNT_ADDRESS (query: config)..."
if ! lcd_smart_query_ok "$LCD" "$FEE_DISCOUNT" '{"config":{}}'; then
  fail_query "fee-discount config at ${FEE_DISCOUNT}"
else
  FD_CFG="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FEE_DISCOUNT" '{"config":{}}')")"
  FD_GOV="$(echo "$FD_CFG" | jq -r '.governance // empty')"
  assert_terra_addr "fee_discount.governance" "$FD_GOV" || true
fi
echo ""

echo "────────────────────────────────────────────────────────────────"
echo "checks: ${CHECKS}   failures: ${FAILURES}"
if [ "$FAILURES" -gt 0 ]; then
  echo "RESULT: FAIL"
  echo ""
  echo "Docs: scripts/qa/README.md; skills/AGENTS_DEPLOY_ENV_ADDRESSES_VERIFY.md" >&2
  exit 1
fi
echo "RESULT: PASS"
echo "Paste this output on the release / launch tracking issue (SEC-H04)."
