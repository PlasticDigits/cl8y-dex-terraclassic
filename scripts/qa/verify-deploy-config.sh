#!/usr/bin/env bash
# Post-deploy config verification: query and assert factory, fee-discount, router, hooks, blacklist.
# GitLab #441 — SEC-H03: scripted post-deploy config checks for release sign-off.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

STAMP_FILE="${QA_DEPLOY_STAMP_FILE:-$REPO_ROOT/.qa-deploy-stamp}"

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

assert_nonempty_terra_addr() {
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

assert_positive_int() {
  local label="$1"
  local value="$2"
  CHECKS=$((CHECKS + 1))
  if [[ "$value" =~ ^[0-9]+$ && "$value" -gt 0 ]]; then
    echo "  [PASS] ${label}=${value}"
    return 0
  fi
  echo "  [FAIL] ${label} must be a positive integer (got: ${value:-<empty>})" >&2
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

assert_min_count() {
  local label="$1"
  local count="$2"
  local min="$3"
  CHECKS=$((CHECKS + 1))
  if [[ "$count" =~ ^[0-9]+$ && "$count" -ge "$min" ]]; then
    echo "  [PASS] ${label} count=${count} (min ${min})"
    return 0
  fi
  echo "  [FAIL] ${label}: expected at least ${min}, got ${count:-<empty>}" >&2
  FAILURES=$((FAILURES + 1))
  return 1
}

assert_bool() {
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

FACTORY=""
ROUTER=""
FEE_DISCOUNT=""
PAIR=""
if [ -f "$REPO_ROOT/indexer/.env" ]; then
  FACTORY="$(read_env_var "$REPO_ROOT/indexer/.env" FACTORY_ADDRESS)"
  ROUTER="$(read_env_var "$REPO_ROOT/indexer/.env" ROUTER_ADDRESS)"
  FEE_DISCOUNT="$(read_env_var "$REPO_ROOT/indexer/.env" FEE_DISCOUNT_ADDRESS)"
fi
if [ -f "$STAMP_FILE" ]; then
  # shellcheck disable=SC1090
  source "$STAMP_FILE"
  PAIR="${pair_address:-}"
  if [ -n "${factory_address:-}" ] && [ -z "$FACTORY" ]; then
    FACTORY="${factory_address}"
  fi
fi
if [ -f "$REPO_ROOT/frontend-dapp/.env.local" ]; then
  [ -z "$FACTORY" ] && FACTORY="$(read_env_var "$REPO_ROOT/frontend-dapp/.env.local" VITE_FACTORY_ADDRESS)"
  [ -z "$ROUTER" ] && ROUTER="$(read_env_var "$REPO_ROOT/frontend-dapp/.env.local" VITE_ROUTER_ADDRESS)"
  [ -z "$FEE_DISCOUNT" ] && FEE_DISCOUNT="$(read_env_var "$REPO_ROOT/frontend-dapp/.env.local" VITE_FEE_DISCOUNT_ADDRESS)"
fi

CLEAN_WALLET="${VERIFY_CONFIG_CLEAN_WALLET:-${TEST_ADDRESS:-terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v}}"
MIN_WHITELISTED="${VERIFY_CONFIG_MIN_WHITELISTED_CODE_IDS:-1}"
MIN_TIERS="${VERIFY_CONFIG_MIN_TIERS:-1}"

_RPC_URL="${TERRA_RPC_URL:-http://127.0.0.1:${DEX_TERRA_RPC_PORT:-26657}}"
if ! localterra_rpc_status_ok "$_RPC_URL"; then
  echo "ERROR: LocalTerra RPC not reachable at ${_RPC_URL}; start compose and deploy first." >&2
  exit 1
fi

if [ -z "$FACTORY" ] || [ -z "$ROUTER" ] || [ -z "$FEE_DISCOUNT" ]; then
  echo "ERROR: FACTORY_ADDRESS, ROUTER_ADDRESS, and FEE_DISCOUNT_ADDRESS required (run make deploy-local)." >&2
  exit 1
fi

HEAD_SHA="$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
echo "=== CL8Y DEX post-deploy config verification (SEC-H03 / GitLab #441) ==="
echo "timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "git_sha: ${HEAD_SHA}"
echo "lcd: ${LCD}"
echo "factory: ${FACTORY}"
echo "router: ${ROUTER}"
echo "fee_discount: ${FEE_DISCOUNT}"
echo ""

# ── 1. Factory config (governance, treasury, default_fee_bps) ─────────────
echo "[1/6] Factory config (query: config)..."
if ! lcd_smart_query_ok "$LCD" "$FACTORY" '{"config":{}}'; then
  fail_query "factory config query"
  FACTORY_CFG='{}'
else
  FACTORY_CFG="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FACTORY" '{"config":{}}')")"
  GOV="$(echo "$FACTORY_CFG" | jq -r '.governance // empty')"
  TREASURY="$(echo "$FACTORY_CFG" | jq -r '.treasury // empty')"
  FEE_BPS="$(echo "$FACTORY_CFG" | jq -r '.default_fee_bps // empty')"
  assert_nonempty_terra_addr "governance" "$GOV" || true
  assert_nonempty_terra_addr "treasury" "$TREASURY" || true
  assert_positive_int "default_fee_bps" "$FEE_BPS" || true
  if [ -n "${VERIFY_CONFIG_EXPECT_GOVERNANCE:-}" ]; then
    assert_eq "governance (expected)" "$GOV" "$VERIFY_CONFIG_EXPECT_GOVERNANCE" || true
  fi
  if [ -n "${VERIFY_CONFIG_EXPECT_TREASURY:-}" ]; then
    assert_eq "treasury (expected)" "$TREASURY" "$VERIFY_CONFIG_EXPECT_TREASURY" || true
  fi
  if [ -n "${VERIFY_CONFIG_EXPECT_DEFAULT_FEE_BPS:-}" ]; then
    assert_eq "default_fee_bps (expected)" "$FEE_BPS" "$VERIFY_CONFIG_EXPECT_DEFAULT_FEE_BPS" || true
  fi
fi
echo ""

# ── 2. Whitelisted CW20 code IDs ───────────────────────────────────────────
echo "[2/6] Factory whitelisted CW20 code IDs..."
if ! lcd_smart_query_ok "$LCD" "$FACTORY" '{"get_whitelisted_code_ids":{"start_after":null,"limit":60}}'; then
  fail_query "get_whitelisted_code_ids"
else
  WL_DOC="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FACTORY" \
    '{"get_whitelisted_code_ids":{"start_after":null,"limit":60}}')")"
  WL_COUNT="$(echo "$WL_DOC" | jq '[.code_ids[]?] | length')"
  assert_min_count "whitelisted_code_ids" "$WL_COUNT" "$MIN_WHITELISTED" || true
  echo "  code_ids: $(echo "$WL_DOC" | jq -c '.code_ids // []')"
fi
echo ""

# ── 3. Fee-discount tiers ──────────────────────────────────────────────────
echo "[3/6] Fee-discount tiers (query: get_tiers)..."
if ! lcd_smart_query_ok "$LCD" "$FEE_DISCOUNT" '{"get_tiers":{}}'; then
  fail_query "fee-discount get_tiers"
else
  TIERS_DOC="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FEE_DISCOUNT" '{"get_tiers":{}}')")"
  TIER_COUNT="$(echo "$TIERS_DOC" | jq '[.tiers[]?] | length')"
  assert_min_count "fee_discount_tiers" "$TIER_COUNT" "$MIN_TIERS" || true
fi
echo ""

# ── 4. Trusted router ──────────────────────────────────────────────────────
echo "[4/6] Trusted router (query: is_trusted_router)..."
TRUSTED_MSG="$(jq -nc --arg addr "$ROUTER" '{is_trusted_router:{addr:$addr}}')"
if ! lcd_smart_query_ok "$LCD" "$FEE_DISCOUNT" "$TRUSTED_MSG"; then
  fail_query "is_trusted_router for ${ROUTER}"
else
  TRUSTED_DOC="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FEE_DISCOUNT" "$TRUSTED_MSG")")"
  IS_TRUSTED="$(echo "$TRUSTED_DOC" | jq -r '.is_trusted // false')"
  assert_bool "is_trusted_router(${ROUTER})" "$IS_TRUSTED" "true" || true
fi
echo ""

# ── 5. Pair hooks on first dual-CW20 pair ──────────────────────────────────
echo "[5/6] Registered hooks on first pair (query: get_hooks)..."
if [ -z "$PAIR" ]; then
  Q_PAIRS="$(lcd_b64_query '{"pairs":{"start_after":null,"limit":60}}')"
  RAW_PAIRS="$(localterra_lcd_curl "$LCD" "/cosmwasm/wasm/v1/contract/${FACTORY}/smart/${Q_PAIRS}")" || true
  if [ -n "$RAW_PAIRS" ]; then
    PAIRS_DOC="$(lcd_decode_smart_data "$RAW_PAIRS")"
    while IFS= read -r row; do
      [ -n "$row" ] || continue
      local_pair="$(echo "$row" | jq -r '.contract_addr')"
      t0="$(echo "$row" | jq -r '.asset_infos[0].token.contract_addr // empty')"
      t1="$(echo "$row" | jq -r '.asset_infos[1].token.contract_addr // empty')"
      if [[ "$t0" =~ ^terra1 && "$t1" =~ ^terra1 ]]; then
        PAIR="$local_pair"
        break
      fi
    done < <(echo "$PAIRS_DOC" | jq -c '.pairs[]? // empty')
  fi
fi

if [ -z "$PAIR" ]; then
  fail_query "resolve first dual-CW20 pair for hook probe"
else
  echo "  pair: ${PAIR}"
  if ! lcd_smart_query_ok "$LCD" "$PAIR" '{"get_hooks":{}}'; then
    fail_query "pair get_hooks"
  else
    HOOKS_DOC="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$PAIR" '{"get_hooks":{}}')")"
    HOOK_COUNT="$(echo "$HOOKS_DOC" | jq '[.hooks[]?] | length')"
    echo "  hooks: $(echo "$HOOKS_DOC" | jq -c '.hooks // []')"
    if [ -n "${VERIFY_CONFIG_EXPECT_HOOK_COUNT:-}" ]; then
      assert_eq "hook_count (expected)" "$HOOK_COUNT" "$VERIFY_CONFIG_EXPECT_HOOK_COUNT" || true
    elif [ "$HOOK_COUNT" -eq 0 ]; then
      CHECKS=$((CHECKS + 1))
      echo "  [PASS] no hooks registered (pool-only launch default)"
    else
      CHECKS=$((CHECKS + 1))
      echo "  [PASS] ${HOOK_COUNT} hook(s) registered (verify against hook policy)"
    fi
  fi
fi
echo ""

# ── 6. Blacklist clean-wallet probe ────────────────────────────────────────
echo "[6/6] Blacklist state for clean wallet (query: blacklist_check)..."
BL_MSG="$(jq -nc --arg wallet "$CLEAN_WALLET" \
  '{blacklist_check:{wallet:$wallet,tokens:[],pair:null,pairs:[]}}')"
if ! lcd_smart_query_ok "$LCD" "$FACTORY" "$BL_MSG"; then
  fail_query "blacklist_check for ${CLEAN_WALLET}"
else
  BL_DOC="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FACTORY" "$BL_MSG")")"
  WALLET_BL="$(echo "$BL_DOC" | jq -r 'if (.wallet_blacklisted | type) == "boolean" then (.wallet_blacklisted | tostring) else "true" end')"
  BLOCKED="$(echo "$BL_DOC" | jq -r 'if (.blocked | type) == "boolean" then (.blocked | tostring) else "true" end')"
  assert_bool "wallet_blacklisted(${CLEAN_WALLET})" "$WALLET_BL" "false" || true
  assert_bool "blocked(${CLEAN_WALLET})" "$BLOCKED" "false" || true
fi
echo ""

echo "────────────────────────────────────────────────────────────────"
echo "checks: ${CHECKS}   failures: ${FAILURES}"
if [ "$FAILURES" -gt 0 ]; then
  echo "RESULT: FAIL"
  echo ""
  echo "Docs: scripts/qa/README.md; skills/AGENTS_DEPLOY_CONFIG_VERIFY.md" >&2
  exit 1
fi
echo "RESULT: PASS"
echo "Paste this output on the release / launch tracking issue (SEC-H03)."
