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

HEALTHY_COUNT="${VERIFY274_HEALTHY_COUNT:-100}"
PAIR_INDEX="${VERIFY274_PAIR_INDEX:-2}"
HEALTHY_BATCH="${VERIFY274_HEALTHY_BATCH:-5}"
MAX_ADJUST_STEPS="${VERIFY274_MAX_ADJUST_STEPS:-256}"
EXPIRED_TAIL_COUNT="${VERIFY274_EXPIRED_TAIL:-5}"
MAX_STEPS_CAP="${VERIFY274_MAX_STEPS:-15}"
MAX_ORDERS="${VERIFY274_MAX_ORDERS:-100}"
BID_ESCROW_RAW="${VERIFY274_BID_ESCROW:-10000}"
HEALTHY_PRICE="${VERIFY274_HEALTHY_PRICE:-2}"
TAIL_PRICE="${VERIFY274_TAIL_PRICE:-0.10}"
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
  # e2e_terrad_tx merges stderr ("gas estimate: …") with JSON on stdout.
  sed -n '/^{/,$p' | tail -1 | jq -r '.txhash // .tx_response.txhash // empty'
}

query_tx_json() {
  local txhash="$1"
  local attempts=0
  local max="${VERIFY274_TX_QUERY_ATTEMPTS:-15}"
  local json=""

  [[ -n "$txhash" ]] || return 1

  while (( attempts < max )); do
    json="$(docker exec "$CONTAINER_NAME" terrad query tx "$txhash" --node "$TERRAD_NODE" --output json 2>/dev/null || true)"
    if [[ -n "$json" ]] && echo "$json" | jq -e '.txhash // .tx_response.txhash // .hash' >/dev/null 2>&1; then
      echo "$json"
      return 0
    fi
    sleep 2
    attempts=$((attempts + 1))
  done
  return 1
}

tx_gas_used() {
  local txhash="$1"
  local json gas
  json="$(query_tx_json "$txhash")" || return 1
  gas="$(echo "$json" | jq -r '.gas_used // .tx_response.gas_used // empty')"
  [[ -n "$gas" && "$gas" =~ ^[0-9]+$ && "$gas" != "0" ]] || return 1
  echo "$gas"
}

tx_wasm_attr() {
  local txhash="$1"
  local key="$2"
  local json
  json="$(query_tx_json "$txhash")" || return 1
  echo "$json" | jq -r --arg k "$key" \
    '[(.events // .logs[0].events // [])[] | select(.type | test("wasm")) | .attributes[] | select(.key == $k) | .value] | last // empty'
}

tx_wasm_order_ids() {
  local txhash="$1"
  local json
  json="$(query_tx_json "$txhash")" || return 1
  echo "$json" | jq -r '[(.events // .logs[0].events // [])[] | select(.type | test("wasm")) | .attributes[] | select(.key == "order_id") | .value] | .[]'
}

max_order_id_from_tx() {
  local txhash="$1"
  tx_wasm_order_ids "$txhash" | sort -n | tail -1
}

bid_book_tail_order_id() {
  local cur next payload
  cur="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$PAIR" '{"order_book_head":{"side":"bid"}}')" | jq -r 'if type == "number" then . else .head_order_id // empty end')"
  [[ -n "$cur" && "$cur" != "null" ]] || return 0
  while :; do
    payload="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$PAIR" "$(jq -nc --argjson id "$cur" '{limit_order:{order_id:$id}}')")")"
    next="$(echo "$payload" | jq -r '.next // empty')"
    [[ -n "$next" && "$next" != "null" ]] || break
    cur="$next"
  done
  echo "$cur"
}

resolve_pair() {
  local pairs_doc pair="" t0="" t1="" idx=0 want="$PAIR_INDEX"
  pairs_doc="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FACTORY" '{"pairs":{"start_after":null,"limit":60}}')")"
  while IFS= read -r row; do
    [[ -n "$row" ]] || continue
    t0="$(echo "$row" | jq -r '.asset_infos[0].token.contract_addr // empty')"
    t1="$(echo "$row" | jq -r '.asset_infos[1].token.contract_addr // empty')"
    if [[ "$t0" =~ ^terra1 && "$t1" =~ ^terra1 ]]; then
      if (( idx == want )); then
        pair="$(echo "$row" | jq -r '.contract_addr')"
        break
      fi
      idx=$((idx + 1))
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
  local hint_after="${5:-}"
  local hook total_escrow txjson txhash
  if [[ -n "$hint_after" ]]; then
    hook="$(jq -nc \
      --argjson exp "$expires" \
      --arg price "$price" \
      --arg amt "$amount" \
      --argjson n "$count" \
      --argjson steps "$MAX_ADJUST_STEPS" \
      --argjson hint "$hint_after" \
      '{place_limit_order_batch:{side:"bid",orders:[range(0; $n) | ({price:$price, amount:$amt, max_adjust_steps:$steps, expires_at:$exp} + (if . == 0 then {hint_after_order_id:$hint} else {} end))]}}')"
  else
    hook="$(jq -nc \
      --argjson exp "$expires" \
      --arg price "$price" \
      --arg amt "$amount" \
      --argjson n "$count" \
      --argjson steps "$MAX_ADJUST_STEPS" \
      '{place_limit_order_batch:{side:"bid",orders:[range(0; $n) | {price:$price, amount:$amt, max_adjust_steps:$steps, expires_at:$exp}]}}')"
  fi
  # Batch CW20 send must equal the sum of per-rung `amount` fields (token1 escrow for bids).
  total_escrow=$((count * amount))
  txjson="$(terrad_tx wasm execute "$TOKEN1" "$(jq -nc \
    --arg pair "$PAIR" \
    --arg amt "$total_escrow" \
    --arg hook "$(echo -n "$hook" | base64 -w0 2>/dev/null || echo -n "$hook" | base64)" \
    '{send:{contract:$pair, amount:$amt, msg:$hook}}')" )"
  txhash="$(echo "$txjson" | tx_hash_from_json)"
  [[ -n "$txhash" ]] || { echo "$txjson" >&2; return 1; }
  sleep 2
  PLACE_TX="$txhash"
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
FAR_EXPIRY=$((NOW_SEC + 1000000))

TAIL_HINT_ORDER_ID=""
if (( HEALTHY_COUNT > 0 )); then
echo "==> Seed $HEALTHY_COUNT healthy far-future bids (head prefix; distinct prices per batch)"
remaining="$HEALTHY_COUNT"
batch_num=0
while (( remaining > 0 )); do
  batch="$HEALTHY_BATCH"
  (( batch > remaining )) && batch="$remaining"
  # Descending price per batch avoids LimitInsertStepsExceeded on deep same-price stacks.
  batch_price="$(awk -v b="$batch_num" 'BEGIN{printf "%.2f", 10.0 - b * 0.5}')"
  echo "    placing batch of $batch healthy bids at price $batch_price..."
  place_bid_batch "$batch" "$batch_price" "$BID_ESCROW_RAW" "$FAR_EXPIRY"
  TAIL_HINT_ORDER_ID="$(max_order_id_from_tx "$PLACE_TX")"
  remaining=$((remaining - batch))
  batch_num=$((batch_num + 1))
done
else
  echo "==> Skip healthy seed (VERIFY274_HEALTHY_COUNT=0); using existing book depth"
  TAIL_HINT_ORDER_ID="$(bid_book_tail_order_id)"
fi

# Recompute short expiry after seeding — a 100-order book can take >45s to place.
NOW_SEC="$(latest_block_unix)"
SHORT_EXPIRY=$((NOW_SEC + EXPIRY_LEAD_SEC))

echo "==> Seed $EXPIRED_TAIL_COUNT expired bids at tail price $TAIL_PRICE (hint_after=${TAIL_HINT_ORDER_ID:-head})"
if [[ -n "$TAIL_HINT_ORDER_ID" ]]; then
  place_bid_batch "$EXPIRED_TAIL_COUNT" "$TAIL_PRICE" "$BID_ESCROW_RAW" "$SHORT_EXPIRY" "$TAIL_HINT_ORDER_ID"
else
  place_bid_batch "$EXPIRED_TAIL_COUNT" "$TAIL_PRICE" "$BID_ESCROW_RAW" "$SHORT_EXPIRY"
fi
TAIL_ORDER_IDS="$(tx_wasm_order_ids "$PLACE_TX" | sort -n | tr '\n' ' ')"
TAIL_ORDER_COUNT="$(echo "$TAIL_ORDER_IDS" | wc -w | tr -d ' ')"
if (( TAIL_ORDER_COUNT >= EXPIRED_TAIL_COUNT )); then
  ok "expired tail placement created $TAIL_ORDER_COUNT orders (ids: ${TAIL_ORDER_IDS})"
else
  bad "expired tail placement created $TAIL_ORDER_COUNT orders (expected $EXPIRED_TAIL_COUNT)"
fi

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

if (( NOW_SEC < SHORT_EXPIRY )); then
  echo ""
  echo "==> Summary ($PASS passed, $FAIL failed)"
  printf '%s\n' "${RESULTS[@]}"
  exit 1
fi

echo "==> CleanLimitBook max_steps=$MAX_STEPS_CAP against ${HEALTHY_COUNT}+${EXPIRED_TAIL_COUNT} book"
execute_clean "$MAX_STEPS_CAP"
GAS="$(tx_gas_used "$CLEAN_TX" || true)"
SCAN_CAPPED="$(tx_wasm_attr "$CLEAN_TX" scan_capped || true)"
CLEANED="$(tx_wasm_attr "$CLEAN_TX" cleaned_count || true)"
RESUME="$(tx_wasm_attr "$CLEAN_TX" resume_cursor || true)"
echo "    tx=$CLEAN_TX gas_used=${GAS:-<unavailable>} scan_capped=$SCAN_CAPPED cleaned_count=$CLEANED resume_cursor=$RESUME"

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

if [[ -z "${CLEAN_TX:-}" ]]; then
  bad "clean tx hash missing — CleanLimitBook never recorded"
elif [[ -n "$GAS" ]] && (( GAS < GAS_CEILING )); then
  ok "gas_used=$GAS < ceiling=$GAS_CEILING (bounded vs ${HEALTHY_COUNT}+ node walk)"
elif [[ -n "$GAS" ]]; then
  bad "gas_used=$GAS exceeds ceiling=$GAS_CEILING — traversal may still scale with book depth"
else
  bad "gas_used unavailable for tx=$CLEAN_TX (query failed or gas missing/zero)"
fi

if [[ -n "$RESUME" ]]; then
  echo "==> Resume clean from resume_cursor until expired tail parked"
  pass=2
  resume="$RESUME"
  total_parked=0
  while (( pass <= 20 )); do
    execute_clean 500 "$resume"
    cleaned="$(tx_wasm_attr "$CLEAN_TX" cleaned_count || true)"
    scan="$(tx_wasm_attr "$CLEAN_TX" scan_capped || true)"
    resume="$(tx_wasm_attr "$CLEAN_TX" resume_cursor || true)"
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
fi

echo ""
echo "==> Summary ($PASS passed, $FAIL failed)"
printf '%s\n' "${RESULTS[@]}"
if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #274 live gas verification passed"
