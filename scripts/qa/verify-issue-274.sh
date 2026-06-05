#!/usr/bin/env bash
# On-chain gas verification for GitLab #274:
# `clean_limit_book` traversal is bounded by `max_steps` independent of book depth.
#
# Proves on a live LocalTerra deploy that:
#   1. A deep prefix of healthy (unparkable) bids does not make clean gas scale with depth
#      when `max_steps` is small — `scan_capped=true`, `cleaned_count=0`.
#   2. `resume_cursor` is emitted and a resumed clean reaches/parks the expired tail.
#   3. Deployed wasm accepts optional `max_steps` on `CleanLimitBook` (schema + attrs).
#
# Refs: smartcontracts/contracts/pair/src/limit_book_clean.rs,
#       smartcontracts/packages/dex-common/src/limit_clean.rs (MAX_CLEAN_SCAN_STEPS=500),
#       docs/contracts-security-audit.md (L15), docs/limit-orders.md.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/lib/lcd-smart-query.sh
source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
# shellcheck source=scripts/lib/e2e-terrad-tx.sh
source "$REPO_ROOT/scripts/lib/e2e-terrad-tx.sh"

CHAIN_ID="${CHAIN_ID:-localterra}"
TERRAD_NODE="${TERRAD_NODE:-http://127.0.0.1:26657}"
CONTAINER_NAME="$(docker compose ps -q localterra 2>/dev/null | head -1)"

HEALTHY_COUNT="${VERIFY274_HEALTHY_COUNT:-250}"
HEALTHY_BATCH="${VERIFY274_HEALTHY_BATCH:-100}"
EXPIRED_TAIL_COUNT="${VERIFY274_EXPIRED_TAIL:-5}"
MAX_STEPS_CAP="${VERIFY274_MAX_STEPS:-15}"
MAX_ORDERS="${VERIFY274_MAX_ORDERS:-100}"
BID_ESCROW_RAW="${VERIFY274_BID_ESCROW:-10000}"
HEALTHY_PRICE="${VERIFY274_HEALTHY_PRICE:-2}"
TAIL_PRICE="${VERIFY274_TAIL_PRICE:-1.5}"
EXPIRY_LEAD_SEC="${VERIFY274_EXPIRY_LEAD_SEC:-45}"
# ~19k gas/step * 15 steps + base; well below walking 250+ nodes (~4.7M+ at 19k/step).
GAS_CEILING="${VERIFY274_GAS_CEILING:-800000}"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

read_env_var() { sed -n "s/^${2}=//p" "$1" 2>/dev/null | head -1; }

if [[ -z "$CONTAINER_NAME" ]]; then
  echo "ERROR: localterra container not running (make start)." >&2
  exit 1
fi

IDX_ENV="$REPO_ROOT/indexer/.env"
FE_ENV="$REPO_ROOT/frontend-dapp/.env.local"
FACTORY="$(read_env_var "$IDX_ENV" FACTORY_ADDRESS)"
LCD="$(read_env_var "$IDX_ENV" LCD_URLS)"; LCD="${LCD%%,*}"
LCD="${LCD:-http://127.0.0.1:1317}"; LCD="${LCD%/}"

if [[ -z "$FACTORY" && -f "$FE_ENV" ]]; then
  FACTORY="$(read_env_var "$FE_ENV" VITE_FACTORY_ADDRESS)"
  LCD="$(read_env_var "$FE_ENV" VITE_TERRA_LCD_URL)"
  LCD="${LCD:-http://127.0.0.1:1317}"; LCD="${LCD%/}"
fi

if [[ -z "$FACTORY" ]]; then
  echo "ERROR: FACTORY address missing — run make deploy-local." >&2
  exit 1
fi

terrad_tx() {
  e2e_terrad_tx "$CONTAINER_NAME" "$@"
}

latest_block_unix() {
  local iso
  iso="$(curl -sf "$LCD/cosmos/base/tendermint/v1beta1/blocks/latest" | jq -r '.block.header.time')"
  date -u -d "$iso" +%s 2>/dev/null || date -u -j -f "%Y-%m-%dT%H:%M:%S" "${iso%%.*}" +%s
}

tx_hash_from_json() {
  jq -r '.txhash // .tx_response.txhash // empty'
}

tx_gas_used() {
  docker exec "$CONTAINER_NAME" terrad query tx "$1" --node "$TERRAD_NODE" --output json 2>/dev/null \
    | jq -r '.gas_used // .tx_response.gas_used // "0"'
}

tx_wasm_attr() {
  local txhash="$1"
  local key="$2"
  docker exec "$CONTAINER_NAME" terrad query tx "$txhash" --node "$TERRAD_NODE" --output json 2>/dev/null \
    | jq -r --arg k "$key" \
      '[(.events // .logs[0].events // [])[] | select(.type | test("wasm")) | .attributes[] | select(.key == $k) | .value] | last // empty'
}

resolve_pair() {
  local pairs_doc pair="" t0="" t1=""
  pairs_doc="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FACTORY" '{"pairs":{"start_after":null,"limit":60}}')")"
  while IFS= read -r row; do
    [[ -n "$row" ]] || continue
    t0="$(echo "$row" | jq -r '.asset_infos[0].token.contract_addr // empty')"
    t1="$(echo "$row" | jq -r '.asset_infos[1].token.contract_addr // empty')"
    if [[ "$t0" =~ ^terra1 && "$t1" =~ ^terra1 ]]; then
      pair="$(echo "$row" | jq -r '.contract_addr')"
      break
    fi
  done < <(echo "$pairs_doc" | jq -c '.pairs[]? // empty')
  if [[ -z "$pair" ]]; then
    echo "ERROR: no dual-CW20 pair on factory $FACTORY." >&2
    exit 1
  fi
  PAIR="$pair"
  TOKEN0="$t0"
  TOKEN1="$t1"
}

place_bid_batch() {
  local count="$1"
  local price="$2"
  local amount="$3"
  local expires="$4"
  local hook total_escrow txjson txhash
  hook="$(jq -nc \
    --argjson exp "$expires" \
    --arg price "$price" \
    --arg amt "$amount" \
    --argjson n "$count" \
    '{place_limit_order_batch:{side:"bid",orders:[range(0; $n) | {price:$price, amount:$amt, max_adjust_steps:64, expires_at:$exp}]}}')"
  total_escrow="$(awk -v c="$count" -v a="$amount" -v p="$price" 'BEGIN{printf "%.0f", c*a*p}')"
  txjson="$(terrad_tx wasm execute "$TOKEN1" "$(jq -nc \
    --arg pair "$PAIR" \
    --arg amt "$total_escrow" \
    --arg hook "$(echo -n "$hook" | base64 -w0 2>/dev/null || echo -n "$hook" | base64)" \
    '{send:{contract:$pair, amount:$amt, msg:$hook}}')" )"
  txhash="$(echo "$txjson" | tx_hash_from_json)"
  [[ -n "$txhash" ]] || { echo "$txjson" >&2; return 1; }
  sleep 2
}

execute_clean() {
  local max_steps="$1"
  local start_hint="${2:-}"
  local msg txjson txhash
  if [[ -n "$start_hint" ]]; then
    msg="$(jq -nc --argjson ms "$max_steps" --argjson mo "$MAX_ORDERS" --argjson sh "$start_hint" \
      '{clean_limit_book:{side:"bid", max_orders:$mo, start_hint:$sh, max_steps:$ms}}')"
  else
    msg="$(jq -nc --argjson ms "$max_steps" --argjson mo "$MAX_ORDERS" \
      '{clean_limit_book:{side:"bid", max_orders:$mo, start_hint:null, max_steps:$ms}}')"
  fi
  txjson="$(terrad_tx wasm execute "$PAIR" "$msg")"
  txhash="$(echo "$txjson" | tx_hash_from_json)"
  [[ -n "$txhash" ]] || { echo "$txjson" >&2; return 1; }
  sleep 2
  CLEAN_TX="$txhash"
}

echo "==> GitLab #274 live gas verification"
echo "    FACTORY=$FACTORY  LCD=$LCD"
resolve_pair
echo "    PAIR=$PAIR  TOKEN0=$TOKEN0  TOKEN1=$TOKEN1"

echo "==> Provision dev wallet CW20 (idempotent)"
bash "$REPO_ROOT/scripts/e2e-provision-dev-wallet.sh"

NOW_SEC="$(latest_block_unix)"
FAR_EXPIRY=$((NOW_SEC + 1_000_000))
SHORT_EXPIRY=$((NOW_SEC + EXPIRY_LEAD_SEC))

echo "==> Seed $HEALTHY_COUNT healthy far-future bids at price $HEALTHY_PRICE (head prefix)"
remaining="$HEALTHY_COUNT"
while (( remaining > 0 )); do
  batch="$HEALTHY_BATCH"
  (( batch > remaining )) && batch="$remaining"
  echo "    placing batch of $batch healthy bids..."
  place_bid_batch "$batch" "$HEALTHY_PRICE" "$BID_ESCROW_RAW" "$FAR_EXPIRY"
  remaining=$((remaining - batch))
done

echo "==> Seed $EXPIRED_TAIL_COUNT expired bids at tail price $TAIL_PRICE"
place_bid_batch "$EXPIRED_TAIL_COUNT" "$TAIL_PRICE" "$BID_ESCROW_RAW" "$SHORT_EXPIRY"

echo "==> Wait for chain time >= $SHORT_EXPIRY"
waited=0
while (( waited < 120 )); do
  NOW_SEC="$(latest_block_unix)"
  (( NOW_SEC >= SHORT_EXPIRY )) && break
  sleep 2
  waited=$((waited + 2))
done
if (( NOW_SEC < SHORT_EXPIRY )); then
  bad "chain time did not reach expires_at within 120s"
else
  ok "chain time past expired tail expires_at"
fi

echo "==> CleanLimitBook max_steps=$MAX_STEPS_CAP against ${HEALTHY_COUNT}+${EXPIRED_TAIL_COUNT} book"
execute_clean "$MAX_STEPS_CAP"
GAS="$(tx_gas_used "$CLEAN_TX")"
SCAN_CAPPED="$(tx_wasm_attr "$CLEAN_TX" scan_capped)"
CLEANED="$(tx_wasm_attr "$CLEAN_TX" cleaned_count)"
RESUME="$(tx_wasm_attr "$CLEAN_TX" resume_cursor)"
echo "    tx=$CLEAN_TX gas_used=$GAS scan_capped=$SCAN_CAPPED cleaned_count=$CLEANED resume_cursor=$RESUME"

if [[ "$SCAN_CAPPED" == "true" ]]; then
  ok "scan_capped=true on capped clean (traversal bounded)"
else
  bad "expected scan_capped=true, got '$SCAN_CAPPED'"
fi

if [[ "$CLEANED" == "0" ]]; then
  ok "cleaned_count=0 — healthy head prefix not parked"
else
  bad "expected cleaned_count=0 on first pass, got '$CLEANED'"
fi

if [[ -n "$RESUME" ]]; then
  ok "resume_cursor emitted ($RESUME)"
else
  bad "resume_cursor missing on scan-capped clean"
fi

if [[ "$GAS" =~ ^[0-9]+$ ]] && (( GAS < GAS_CEILING )); then
  ok "gas_used=$GAS < ceiling=$GAS_CEILING (bounded vs ${HEALTHY_COUNT}+ node walk)"
else
  bad "gas_used=$GAS exceeds ceiling=$GAS_CEILING — traversal may still scale with book depth"
fi

echo "==> Resume clean from resume_cursor until expired tail parked"
pass=2
resume="$RESUME"
total_parked=0
while (( pass <= 20 )); do
  execute_clean 500 "$resume"
  cleaned="$(tx_wasm_attr "$CLEAN_TX" cleaned_count)"
  scan="$(tx_wasm_attr "$CLEAN_TX" scan_capped)"
  resume="$(tx_wasm_attr "$CLEAN_TX" resume_cursor)"
  echo "    pass=$pass tx=$CLEAN_TX cleaned=$cleaned scan_capped=$scan resume=$resume"
  total_parked=$((total_parked + cleaned))
  if [[ "$scan" != "true" && -z "$resume" ]]; then
    break
  fi
  ((pass++))
done

if (( total_parked >= EXPIRED_TAIL_COUNT )); then
  ok "resumed clean parked $total_parked expired tail orders (expected >= $EXPIRED_TAIL_COUNT)"
else
  bad "resumed clean parked only $total_parked (expected >= $EXPIRED_TAIL_COUNT)"
fi

echo ""
echo "==> Summary ($PASS passed, $FAIL failed)"
printf '%s\n' "${RESULTS[@]}"
if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #274 live gas verification passed"
