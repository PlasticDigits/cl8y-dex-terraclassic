#!/usr/bin/env bash
# On-chain E2E verification for GitLab #238 (gap H1):
# HybridSimulation CL8Y fee-discount parity with the execute path.
#
# Proves on a live LocalTerra deploy that:
#   1. The deployed pair WASM ACCEPTS the optional `trader` field on
#      `hybrid_simulation` (the prior verification blocker was stale wasm
#      rejecting `trader` with "unknown field trader").
#   2. A registered, discounted trader gets a STRICTLY larger `return_amount`
#      from the sim than an undiscounted (no-`trader`) sim.
#   3. The executed swap output for that trader EQUALS the discounted sim
#      `return_amount` computed against the same pre-swap reserves (quote ==
#      execution; invariant L8).
#   4. Router `simulate_swap_operations` accepts `trader` and reflects the
#      per-hop discount.
#   5. Indexer `GET /api/v1/route/solve?...&trader=` accepts `trader`, returns
#      a quote, and the trader-aware quote is >= the undiscounted quote.
#
# Read-only sim invariant: discount queries never mutate state (no deregister).
#
# Refs: docs/contracts-security-audit.md (L8), docs/adr/0001-hybrid-quoting-and-routing.md,
#       skills/AGENTS_HYBRID_QUOTING.md, skills/AGENTS_FEE_DISCOUNT_TIERS.md,
#       skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/lib/lcd-smart-query.sh
source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"

# LocalTerra dev account (governance, CL8Y minter); see docker/init-chain.sh + deploy-dex-local.sh.
TEST_ADDRESS="${TEST_ADDRESS:-terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v}"
CHAIN_ID="${CHAIN_ID:-localterra}"
TERRAD_NODE="${TERRAD_NODE:-http://127.0.0.1:26657}"
CONTAINER_NAME="$(docker compose ps -q localterra 2>/dev/null | head -1)"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()   { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad()  { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }

read_env_var() { sed -n "s/^${2}=//p" "$1" 2>/dev/null | head -1; }

# ── Resolve addresses + endpoints ────────────────────────────────────────
IDX_ENV="$REPO_ROOT/indexer/.env"
FE_ENV="$REPO_ROOT/frontend-dapp/.env.local"

FACTORY="$(read_env_var "$IDX_ENV" FACTORY_ADDRESS)"
ROUTER="$(read_env_var "$IDX_ENV" ROUTER_ADDRESS)"
FEE_DISCOUNT="$(read_env_var "$IDX_ENV" FEE_DISCOUNT_ADDRESS)"
LCD="$(read_env_var "$IDX_ENV" LCD_URLS)"; LCD="${LCD%%,*}"
API_PORT="$(read_env_var "$IDX_ENV" API_PORT)"
CL8Y="$(read_env_var "$FE_ENV" VITE_CL8Y_TOKEN_ADDRESS)"

LCD="${LCD:-http://127.0.0.1:1317}"; LCD="${LCD%/}"
API_PORT="${API_PORT:-3001}"
INDEXER="http://127.0.0.1:${API_PORT}"

if [ -z "$FACTORY" ] || [ -z "$ROUTER" ] || [ -z "$FEE_DISCOUNT" ] || [ -z "$CL8Y" ]; then
  echo "ERROR: missing addresses (FACTORY=$FACTORY ROUTER=$ROUTER FEE_DISCOUNT=$FEE_DISCOUNT CL8Y=$CL8Y). Run make deploy-local." >&2
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
wait_tx() { sleep 4; }

# Authoritative swap output: the pair emits a `return_amount` wasm attribute.
# We compare sim vs this (not CW20 balance delta) because the fee `treasury` can
# be the same wallet as `trader` on localnet — the treasury also receives the
# `commission_amount`, which would inflate a naive balance delta.
tx_return_amount() {
  # terrad v4 (SDK 0.53) exposes ABCI events at top-level `.events`; v3.x nested them under
  # `.logs[0].events` (empty on v4). Read `.events` first, fall back to `.logs` for old nodes.
  docker exec "$CONTAINER_NAME" terrad query tx "$1" --node "$TERRAD_NODE" --output json 2>/dev/null \
    | jq -r '[(.events // .logs[0].events // [])[] | select(.type|test("wasm")) | .attributes[] | select(.key=="return_amount") | .value] | last // "ERR"'
}

# Resolve first dual-CW20 pair + its two token addresses from the factory.
echo "==> Resolving dual-CW20 pair from factory ${FACTORY}..."
PAIRS_RAW="$(lcd_smart_query_raw "$LCD" "$FACTORY" '{"pairs":{"start_after":null,"limit":60}}')"
PAIRS_DOC="$(lcd_decode_smart_data "$PAIRS_RAW")"
PAIR=""; TOKEN0=""; TOKEN1=""
while IFS= read -r row; do
  [ -n "$row" ] || continue
  t0="$(echo "$row" | jq -r '.asset_infos[0].token.contract_addr // empty')"
  t1="$(echo "$row" | jq -r '.asset_infos[1].token.contract_addr // empty')"
  if [[ "$t0" =~ ^terra1 && "$t1" =~ ^terra1 ]]; then
    PAIR="$(echo "$row" | jq -r '.contract_addr')"; TOKEN0="$t0"; TOKEN1="$t1"; break
  fi
done < <(echo "$PAIRS_DOC" | jq -c '.pairs[]? // empty')

if [ -z "$PAIR" ]; then echo "ERROR: no dual-CW20 pair found." >&2; exit 1; fi
echo "    PAIR=$PAIR"
echo "    TOKEN0(offer)=$TOKEN0  TOKEN1(ask)=$TOKEN1  CL8Y=$CL8Y"

# Swap input: small vs reserves so the fee-discount delta is clearly visible.
OFFER_AMT="${OFFER_AMT:-1000000000}"   # 1e9 base units
pool_only() { printf '{"pool_input":"%s","book_input":"0","max_maker_fills":1,"book_start_hint":null}' "$1"; }

asset_json() { printf '{"info":{"token":{"contract_addr":"%s"}},"amount":"%s"}' "$1" "$2"; }

sim_query() {  # $1 trader-or-empty -> prints return_amount (or "ERR")
  local trader_field=""
  if [ -n "$1" ]; then trader_field="$(printf '"%s"' "$1")"; else trader_field="null"; fi
  local msg
  msg="$(printf '{"hybrid_simulation":{"offer_asset":%s,"hybrid":%s,"trader":%s,"sender":null}}' \
    "$(asset_json "$TOKEN0" "$OFFER_AMT")" "$(pool_only "$OFFER_AMT")" "$trader_field")"
  if ! lcd_smart_query_ok "$LCD" "$PAIR" "$msg"; then echo "ERR"; return; fi
  lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$PAIR" "$msg")" | jq -r '.return_amount // "ERR"'
}

router_sim() {  # $1 trader-or-empty -> prints amount (or "ERR")
  local trader_field="null"
  [ -n "$1" ] && trader_field="$(printf '"%s"' "$1")"
  local ops msg
  ops="$(printf '[{"terra_swap":{"offer_asset_info":{"token":{"contract_addr":"%s"}},"ask_asset_info":{"token":{"contract_addr":"%s"}},"hybrid":null}}]' "$TOKEN0" "$TOKEN1")"
  msg="$(printf '{"simulate_swap_operations":{"offer_amount":"%s","operations":%s,"trader":%s,"sender":null}}' "$OFFER_AMT" "$ops" "$trader_field")"
  if ! lcd_smart_query_ok "$LCD" "$ROUTER" "$msg"; then echo "ERR"; return; fi
  lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$ROUTER" "$msg")" | jq -r '.amount // "ERR"'
}

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #238 — on-chain hybrid sim fee-discount parity"
echo "════════════════════════════════════════════════════════════════"

# ── 1. Schema acceptance: pair hybrid_simulation accepts `trader` ────────
echo ""
echo "[1] Pair hybrid_simulation accepts optional trader (prior blocker)..."
RET_NOTRADER="$(sim_query "")"
RET_WITHTRADER_SCHEMA="$(sim_query "$TEST_ADDRESS")"
if [ "$RET_NOTRADER" != "ERR" ] && [ "$RET_WITHTRADER_SCHEMA" != "ERR" ]; then
  ok "pair hybrid_simulation accepts trader field (no 'unknown field trader')"
else
  bad "pair hybrid_simulation rejected trader (no-trader=$RET_NOTRADER with-trader=$RET_WITHTRADER_SCHEMA) — stale wasm?"
fi

# ── 2. Register a discounted tier for the dev wallet ─────────────────────
echo ""
echo "[2] Registering CL8Y fee-discount tier for trader ${TEST_ADDRESS}..."
# Highest self-register tier (9 = 9500 bps) needs 7500 CL8Y = 7.5e21 base units.
# LocalTerra CL8Y is TCL8Y (18 decimals); mint plenty so the wallet qualifies (GitLab #383).
TIER_ID="${TIER_ID:-9}"
MINT_AMT="${MINT_AMT:-10000000000000000000000}"   # 1e22
DISC_RAW="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FEE_DISCOUNT" \
  "$(printf '{"get_discount":{"trader":"%s","sender":"%s"}}' "$TEST_ADDRESS" "$TEST_ADDRESS")")")"
CUR_DISC="$(echo "$DISC_RAW" | jq -r '.discount_bps // 0')"
if [ "${CUR_DISC:-0}" -gt 0 ]; then
  echo "    already registered (discount_bps=$CUR_DISC); skipping mint/register"
else
  echo "    minting CL8Y to dev wallet..."
  terrad_tx wasm execute "$CL8Y" "$(printf '{"mint":{"recipient":"%s","amount":"%s"}}' "$TEST_ADDRESS" "$MINT_AMT")" >/dev/null; wait_tx
  echo "    registering tier ${TIER_ID}..."
  terrad_tx wasm execute "$FEE_DISCOUNT" "$(printf '{"register":{"tier_id":%s}}' "$TIER_ID")" >/dev/null; wait_tx
  DISC_RAW="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FEE_DISCOUNT" \
    "$(printf '{"get_discount":{"trader":"%s","sender":"%s"}}' "$TEST_ADDRESS" "$TEST_ADDRESS")")")"
  CUR_DISC="$(echo "$DISC_RAW" | jq -r '.discount_bps // 0')"
fi
if [ "${CUR_DISC:-0}" -gt 0 ]; then
  ok "fee-discount registry returns discount_bps=$CUR_DISC for dev wallet"
else
  bad "fee-discount registry returned no discount (discount_bps=$CUR_DISC); cannot verify parity"
fi

# ── 3. Discount applied: sim(trader) > sim(no trader) ────────────────────
echo ""
echo "[3] Discounted sim quotes more output than undiscounted..."
RET_NOTRADER="$(sim_query "")"
RET_WITHTRADER="$(sim_query "$TEST_ADDRESS")"
echo "    return_amount  no-trader=$RET_NOTRADER  with-trader=$RET_WITHTRADER"
if [ "$RET_NOTRADER" != "ERR" ] && [ "$RET_WITHTRADER" != "ERR" ] \
   && [ "$(echo "$RET_WITHTRADER > $RET_NOTRADER" | bc)" = "1" ]; then
  ok "discounted sim return_amount ($RET_WITHTRADER) > undiscounted ($RET_NOTRADER)"
else
  bad "discounted sim did not exceed undiscounted (with=$RET_WITHTRADER no=$RET_NOTRADER)"
fi

# ── 4. Execute parity: swap output == discounted sim (same snapshot) ─────
echo ""
echo "[4] Executed swap return_amount == discounted sim return_amount (same reserves)..."
EXPECTED="$(sim_query "$TEST_ADDRESS")"
# CW20 Send to pair with pool-only Swap hook; trader=null -> discount uses CW20 sender (dev wallet).
SWAP_HOOK="$(printf '{"swap":{"belief_price":null,"max_spread":"0.5","to":null,"deadline":null,"hybrid":%s,"trader":null}}' "$(pool_only "$OFFER_AMT")" | base64 -w0)"
SEND_MSG="$(printf '{"send":{"contract":"%s","amount":"%s","msg":"%s"}}' "$PAIR" "$OFFER_AMT" "$SWAP_HOOK")"
SWAP_TX="$(terrad_tx wasm execute "$TOKEN0" "$SEND_MSG" | jq -r '.txhash')"; wait_tx
ACTUAL="$(tx_return_amount "$SWAP_TX")"
echo "    sim return_amount=$EXPECTED  executed return_amount=$ACTUAL  (tx ${SWAP_TX:0:12})"
if [ "$EXPECTED" != "ERR" ] && [ "$ACTUAL" != "ERR" ] && [ "$ACTUAL" = "$EXPECTED" ]; then
  ok "executed swap return_amount ($ACTUAL) == discounted sim return_amount ($EXPECTED)"
else
  bad "execute != sim (sim=$EXPECTED executed=$ACTUAL)"
fi

# ── 5. Router sim accepts trader and reflects discount ───────────────────
echo ""
echo "[5] Router simulate_swap_operations accepts trader + reflects discount..."
RSIM_NO="$(router_sim "")"
RSIM_YES="$(router_sim "$TEST_ADDRESS")"
echo "    router amount  no-trader=$RSIM_NO  with-trader=$RSIM_YES"
if [ "$RSIM_NO" != "ERR" ] && [ "$RSIM_YES" != "ERR" ] \
   && [ "$(echo "$RSIM_YES >= $RSIM_NO" | bc)" = "1" ] \
   && [ "$(echo "$RSIM_YES > $RSIM_NO" | bc)" = "1" ]; then
  ok "router sim with trader ($RSIM_YES) > without ($RSIM_NO)"
else
  bad "router sim trader handling failed (with=$RSIM_YES no=$RSIM_NO)"
fi

# ── 5b. Reverse sim accepts trader + reflects discount ───────────────────
echo ""
echo "[5b] Pair hybrid_reverse_simulation accepts trader + reflects discount..."
ASK_AMT="${ASK_AMT:-500000000}"
rev_sim() {  # $1 trader-or-empty -> prints offer_amount (or ERR)
  local tf="null"; [ -n "$1" ] && tf="$(printf '"%s"' "$1")"
  local msg
  msg="$(printf '{"hybrid_reverse_simulation":{"ask_asset":%s,"hybrid":{"pool_input":"1","book_input":"0","max_maker_fills":1,"book_start_hint":null},"trader":%s,"sender":null}}' \
    "$(asset_json "$TOKEN1" "$ASK_AMT")" "$tf")"
  if ! lcd_smart_query_ok "$LCD" "$PAIR" "$msg"; then echo "ERR"; return; fi
  lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$PAIR" "$msg")" | jq -r '.offer_amount // "ERR"'
}
REV_NO="$(rev_sim "")"
REV_YES="$(rev_sim "$TEST_ADDRESS")"
echo "    reverse offer_amount  no-trader=$REV_NO  with-trader=$REV_YES"
if [ "$REV_NO" != "ERR" ] && [ "$REV_YES" != "ERR" ] \
   && [ "$(echo "$REV_YES < $REV_NO" | bc)" = "1" ]; then
  ok "discounted reverse sim needs less offer ($REV_YES) than undiscounted ($REV_NO)"
else
  bad "reverse sim trader handling failed (with=$REV_YES no=$REV_NO)"
fi

# ── 6. Indexer route/solve accepts trader ────────────────────────────────
echo ""
echo "[6] Indexer GET /route/solve accepts trader + returns a quote..."
RS_BASE="${INDEXER}/api/v1/route/solve?token_in=${TOKEN0}&token_out=${TOKEN1}&amount_in=${OFFER_AMT}"
RS_NO="$(curl -sf "$RS_BASE" 2>/dev/null || echo '{}')"
RS_YES="$(curl -sf "${RS_BASE}&trader=${TEST_ADDRESS}" 2>/dev/null || echo '{}')"
OUT_NO="$(echo "$RS_NO" | jq -r '.estimated_amount_out // .amount_out // "ERR"')"
OUT_YES="$(echo "$RS_YES" | jq -r '.estimated_amount_out // .amount_out // "ERR"')"
echo "    route/solve estimated_amount_out  no-trader=$OUT_NO  with-trader=$OUT_YES"
if [ "$OUT_YES" != "ERR" ] && [ "$OUT_NO" != "ERR" ] \
   && [ "$(echo "$OUT_YES >= $OUT_NO" | bc)" = "1" ]; then
  ok "indexer route/solve accepts trader; trader quote ($OUT_YES) >= undiscounted ($OUT_NO)"
else
  bad "indexer route/solve trader handling failed (with=$OUT_YES no=$OUT_NO); raw_with=$(echo "$RS_YES" | jq -c '.error? // .')"
fi

# ── Summary ──────────────────────────────────────────────────────────────
echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: ${PASS} passed, ${FAIL} failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"
[ "$FAIL" -eq 0 ]
