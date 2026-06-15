#!/usr/bin/env bash
# On-chain verification for GitLab #383 — LocalTerra TCL8Y (18-decimal CL8Y proxy).
#
# Proves on a live LocalTerra deploy that:
#   1. VITE_CL8Y_TOKEN_ADDRESS is TCL8Y with 18 decimals (not 6-decimal EMBER).
#   2. Fee-discount contract cl8y_token matches VITE_CL8Y_TOKEN_ADDRESS.
#   3. Tier-1 self-registration succeeds (FT-3).
#   4. Deregister succeeds for self-registered tier (FT-4).
#
# Refs: docs/reference/fee-discount-tiers.md, skills/AGENTS_FEE_DISCOUNT_TIERS.md
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/lib/lcd-smart-query.sh
source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
# shellcheck source=scripts/lib/terrad-wait-tx.sh
source "$REPO_ROOT/scripts/lib/terrad-wait-tx.sh"

TEST_ADDRESS="${TEST_ADDRESS:-terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v}"
CHAIN_ID="${CHAIN_ID:-localterra}"
TERRAD_NODE="${TERRAD_NODE:-http://127.0.0.1:26657}"
CONTAINER_NAME="$(docker compose ps -q localterra 2>/dev/null | head -1)"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }

read_env_var() { sed -n "s/^${2}=//p" "$1" 2>/dev/null | head -1; }

IDX_ENV="$REPO_ROOT/indexer/.env"
FE_ENV="$REPO_ROOT/frontend-dapp/.env.local"

FACTORY="$(read_env_var "$IDX_ENV" FACTORY_ADDRESS)"
FEE_DISCOUNT="$(read_env_var "$IDX_ENV" FEE_DISCOUNT_ADDRESS)"
LCD="$(read_env_var "$IDX_ENV" LCD_URLS)"; LCD="${LCD%%,*}"
CL8Y="$(read_env_var "$FE_ENV" VITE_CL8Y_TOKEN_ADDRESS)"

LCD="${LCD:-http://127.0.0.1:1317}"; LCD="${LCD%/}"

if [ -z "$FACTORY" ] || [ -z "$FEE_DISCOUNT" ] || [ -z "$CL8Y" ]; then
  echo "ERROR: missing addresses (FACTORY=$FACTORY FEE_DISCOUNT=$FEE_DISCOUNT CL8Y=$CL8Y). Run make deploy-local." >&2
  exit 1
fi
if [ -z "$CONTAINER_NAME" ]; then
  echo "ERROR: localterra container not running (make start)." >&2
  exit 1
fi

terrad_tx() {
  docker exec "$CONTAINER_NAME" terrad tx "$@" \
    --from test1 --keyring-backend test --chain-id "$CHAIN_ID" \
    --gas auto --gas-adjustment 1.3 --fees 500000000uluna \
    --node "$TERRAD_NODE" --broadcast-mode sync -y --output json
}
terrad_query() {
  docker exec "$CONTAINER_NAME" terrad query "$@" \
    --node "$TERRAD_NODE" \
    --output json
}
wait_tx() {
  local tx_hash="$1"
  terrad_wait_tx_inclusion "$CONTAINER_NAME" "$tx_hash" "$TERRAD_NODE" 90
}

query_registration() {
  terrad_query wasm contract-state smart "$FEE_DISCOUNT" \
    "$(printf '{"get_registration":{"trader":"%s"}}' "$TEST_ADDRESS")" \
    | jq '.data'
}

poll_registration() {
  local expect_registered="$1"
  local tries=12
  local reg=""
  while [ "$tries" -gt 0 ]; do
    reg="$(query_registration)"
    if echo "$reg" | jq -e 'type == "object" and (.registered | type) == "boolean"' >/dev/null 2>&1; then
      local ok
      ok="$(echo "$reg" | jq -r '.registered')"
      if [ "$ok" = "$expect_registered" ]; then
        echo "$reg"
        return 0
      fi
    fi
    tries=$((tries - 1))
    sleep 2
  done
  echo "$reg"
  return 1
}

token_info() {
  lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$1" '{"token_info":{}}')"
}

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #383 — LocalTerra TCL8Y fee-discount proxy"
echo "════════════════════════════════════════════════════════════════"

# ── 1. TCL8Y has 18 decimals ───────────────────────────────────────────
echo ""
echo "[1] TCL8Y token_info: 18 decimals, symbol TCL8Y..."
INFO="$(token_info "$CL8Y")"
DEC="$(echo "$INFO" | jq -r '.decimals // empty')"
SYM="$(echo "$INFO" | jq -r '.symbol // empty')"
SUPPLY="$(echo "$INFO" | jq -r '.total_supply // "0"')"
echo "    CL8Y=$CL8Y symbol=$SYM decimals=$DEC total_supply=$SUPPLY"
if [ "$DEC" = "18" ] && [ "$SYM" = "TCL8Y" ]; then
  ok "TCL8Y has 18 decimals (not 6-decimal EMBER)"
else
  bad "CL8Y token is not TCL8Y/18-decimal (symbol=$SYM decimals=$DEC)"
fi

# Tier 9 needs 7.5e21; deploy seeds 1e25 to test1.
if [ "$SUPPLY" != "0" ] && [ "$(echo "$SUPPLY >= 7500000000000000000000" | bc)" = "1" ]; then
  ok "TCL8Y total supply supports tier-9 minimum (>= 7.5e21)"
else
  bad "TCL8Y total supply too low for tier testing (supply=$SUPPLY)"
fi

# ── 2. CL8Y is not EMBER (first factory pair token) ─────────────────────
echo ""
echo "[2] VITE_CL8Y_TOKEN_ADDRESS is not EMBER (6-decimal trading token)..."
PAIRS_RAW="$(lcd_smart_query_raw "$LCD" "$FACTORY" '{"pairs":{"start_after":null,"limit":5}}')"
PAIRS_DOC="$(lcd_decode_smart_data "$PAIRS_RAW")"
EMBER="$(echo "$PAIRS_DOC" | jq -r '.pairs[0].asset_infos[0].token.contract_addr // empty')"
if [ -n "$EMBER" ] && [ "$CL8Y" != "$EMBER" ]; then
  ok "VITE_CL8Y_TOKEN_ADDRESS differs from EMBER ($EMBER)"
else
  bad "VITE_CL8Y_TOKEN_ADDRESS still points at EMBER or factory pair unresolved"
fi

# ── 3. Fee-discount cl8y_token matches env ───────────────────────────────
echo ""
echo "[3] Fee-discount config cl8y_token matches VITE_CL8Y_TOKEN_ADDRESS..."
CFG="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FEE_DISCOUNT" '{"config":{}}')")"
ON_CHAIN_CL8Y="$(echo "$CFG" | jq -r '.cl8y_token // empty')"
echo "    on-chain cl8y_token=$ON_CHAIN_CL8Y"
if [ "$ON_CHAIN_CL8Y" = "$CL8Y" ]; then
  ok "fee-discount cl8y_token matches VITE_CL8Y_TOKEN_ADDRESS"
else
  bad "fee-discount cl8y_token mismatch (on-chain=$ON_CHAIN_CL8Y env=$CL8Y)"
fi

# ── 4. Tier-1 register (FT-3) ────────────────────────────────────────────
echo ""
echo "[4] Tier-1 self-registration (FT-3)..."
TIER1_MIN="1000000000000000000"
BAL_RAW="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$CL8Y" \
  "$(printf '{"balance":{"address":"%s"}}' "$TEST_ADDRESS")")")"
BAL="$(echo "$BAL_RAW" | jq -r '.balance // "0"')"
echo "    test1 TCL8Y balance=$BAL (tier-1 min=$TIER1_MIN)"
if [ "$(echo "$BAL >= $TIER1_MIN" | bc)" != "1" ]; then
  echo "    minting TCL8Y to test1..."
  MINT_TX="$(terrad_tx wasm execute "$CL8Y" \
    "$(printf '{"mint":{"recipient":"%s","amount":"10000000000000000000000"}}' "$TEST_ADDRESS")" | jq -r '.txhash')"
  wait_tx "$MINT_TX"
fi

# Deregister first if already registered (idempotent re-run).
REG_BEFORE="$(query_registration)"
if [ "$(echo "$REG_BEFORE" | jq -r '.registered // false')" = "true" ]; then
  tier_before="$(echo "$REG_BEFORE" | jq -r '.tier_id // 255')"
  if [ "$tier_before" != "0" ] && [ "$tier_before" != "255" ]; then
    echo "    deregistering existing tier $tier_before for clean FT-3 run..."
    CLEAN_TX="$(terrad_tx wasm execute "$FEE_DISCOUNT" '{"deregister":{}}' | jq -r '.txhash')"
    wait_tx "$CLEAN_TX"
  fi
fi

REG_TX="$(terrad_tx wasm execute "$FEE_DISCOUNT" '{"register":{"tier_id":1}}' | jq -r '.txhash')"
wait_tx "$REG_TX"
REG="$(poll_registration true || poll_registration true)"
REG_OK="$(echo "$REG" | jq -r 'if (.registered | type) == "boolean" then (.registered | tostring) else "false" end')"
REG_TIER="$(echo "$REG" | jq -r '.tier_id // empty')"
echo "    register tx=$REG_TX registered=$REG_OK tier_id=$REG_TIER"
if [ "$REG_OK" = "true" ] && [ "$REG_TIER" = "1" ]; then
  ok "tier-1 registration succeeded (FT-3)"
else
  bad "tier-1 registration failed (registered=$REG_OK tier_id=$REG_TIER)"
fi

# ── 5. Deregister (FT-4) ───────────────────────────────────────────────
echo ""
echo "[5] Deregister self-registered tier (FT-4)..."
DEREG_TX="$(terrad_tx wasm execute "$FEE_DISCOUNT" '{"deregister":{}}' | jq -r '.txhash')"
wait_tx "$DEREG_TX"
REG_AFTER="$(poll_registration false || query_registration)"
AFTER_OK="$(echo "$REG_AFTER" | jq -r 'if (.registered | type) == "boolean" then (.registered | tostring) else "unknown" end')"
echo "    deregister tx=$DEREG_TX registered=$AFTER_OK"
if [ "$AFTER_OK" = "false" ]; then
  ok "deregister succeeded (FT-4)"
else
  bad "deregister failed (still registered=$AFTER_OK)"
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: ${PASS} passed, ${FAIL} failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"
[ "$FAIL" -eq 0 ]
