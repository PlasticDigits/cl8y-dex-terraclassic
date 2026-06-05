#!/usr/bin/env bash
# LocalTerra gas benchmark for GitLab #309 — MAX_EXPIRED_PARKS_PER_SWAP vs 15M envelope.
#
# Methodology:
#   - Optimized wasm from `make build-optimized && make deploy-local`
#   - Synthetic bid book: N expired orders at head (short expires_at), book-only hybrid swap
#   - Records gas_used, wasm event count, serialized tx bytes per N in SWEEP
#   - Verifies on-chain cap attrs at N > MAX_EXPIRED_PARKS_PER_SWAP (default 15)
#
# Refs: smartcontracts/packages/dex-common/src/pair.rs (MAX_EXPIRED_PARKS_PER_SWAP),
#       docs/limit-orders.md § Expired-park benchmark (#309),
#       skills/AGENTS_TERRACLASSIC_GAS.md rule 16.
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

SWEEP="${VERIFY309_SWEEP:-1,5,10,15,20,25,30}"
# Start above deploy smoke-test pairs (indices 0–1) and leave headroom per sweep case.
PAIR_INDEX_BASE="${VERIFY309_PAIR_INDEX:-10}"
MAX_ADJUST_STEPS="${VERIFY309_MAX_ADJUST_STEPS:-32}"
BID_ESCROW_RAW="${VERIFY309_BID_ESCROW:-5000}"
BOOK_INPUT_RAW="${VERIFY309_BOOK_INPUT:-50000}"
MAX_MAKER_FILLS="${VERIFY309_MAX_MAKER_FILLS:-8}"
EXPIRY_LEAD_SEC="${VERIFY309_EXPIRY_LEAD_SEC:-45}"
# 15M dApp ceiling with ~20% headroom (GitLab #309 acceptance).
GAS_CEILING="${VERIFY309_GAS_CEILING:-12000000}"
# Terra Classic wasm tx size limit (bytes).
TX_BYTES_CEILING="${VERIFY309_TX_BYTES_CEILING:-1048576}"

PASS=0
FAIL=0
declare -a RESULTS=()
declare -a BENCH_ROWS=()

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
  sed -n '/^{/,$p' | tail -1 | jq -r '.txhash // .tx_response.txhash // empty'
}

query_tx_json() {
  local txhash="$1"
  local attempts=0
  local max="${VERIFY309_TX_QUERY_ATTEMPTS:-15}"
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

tx_wasm_event_count() {
  local txhash="$1"
  local action="$2"
  local json
  json="$(query_tx_json "$txhash")" || return 1
  echo "$json" | jq -r --arg a "$action" \
    '[(.events // .logs[0].events // [])[] | select(.type | test("wasm")) | .attributes[] | select(.key == "action" and .value == $a)] | length'
}

tx_serialized_bytes() {
  local txhash="$1"
  local json len
  json="$(query_tx_json "$txhash")" || return 1
  len="$(echo "$json" | jq -r '.tx // .tx_response.tx // empty' | wc -c | tr -d ' ')"
  [[ -n "$len" && "$len" =~ ^[0-9]+$ && "$len" != "0" ]] || return 1
  echo "$len"
}

resolve_pair() {
  local want="${1:-$PAIR_INDEX_BASE}"
  local pairs_doc pair="" t0="" t1="" idx=0
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

place_expired_bids() {
  local count="$1"
  local expires="$2"
  local remaining="$count"
  local offset=0
  local batch_size batch hook total_escrow txjson txhash

  # Factory default max_batch_rungs is 20 — chunk larger stacks.
  while (( remaining > 0 )); do
    batch_size=20
    (( batch_size > remaining )) && batch_size="$remaining"
    hook="$(jq -nc \
      --argjson exp "$expires" \
      --arg amt "$BID_ESCROW_RAW" \
      --argjson n "$batch_size" \
      --argjson off "$offset" \
      --argjson steps "$MAX_ADJUST_STEPS" \
      '{place_limit_order_batch:{side:"bid",orders:[range(0; $n) | {price:((100 + $off + .) / 100 | tostring), amount:$amt, max_adjust_steps:$steps, expires_at:$exp}]}}')"
    total_escrow=$((batch_size * BID_ESCROW_RAW))
    txjson="$(terrad_tx wasm execute "$TOKEN1" "$(jq -nc \
      --arg pair "$PAIR" \
      --arg amt "$total_escrow" \
      --arg hook "$(echo -n "$hook" | base64 -w0 2>/dev/null || echo -n "$hook" | base64)" \
      '{send:{contract:$pair, amount:$amt, msg:$hook}}')" )"
    txhash="$(echo "$txjson" | tx_hash_from_json)"
    [[ -n "$txhash" ]] || { echo "$txjson" >&2; return 1; }
    sleep 2
    PLACE_TX="$txhash"
    remaining=$((remaining - batch_size))
    offset=$((offset + batch_size))
  done
}

hybrid_swap_book_only() {
  local book_input="$1"
  local swap_hook txjson txhash
  swap_hook="$(jq -nc \
    --argjson book "$book_input" \
    --argjson fills "$MAX_MAKER_FILLS" \
    '{swap:{belief_price:null,max_spread:"1",to:null,deadline:null,hybrid:{pool_input:"0",book_input:($book|tostring),max_maker_fills:$fills,book_start_hint:null},trader:null}}')"
  txjson="$(terrad_tx wasm execute "$TOKEN0" "$(jq -nc \
    --arg pair "$PAIR" \
    --arg amt "$book_input" \
    --arg hook "$(echo -n "$swap_hook" | base64 -w0 2>/dev/null || echo -n "$swap_hook" | base64)" \
    '{send:{contract:$pair, amount:$amt, msg:$hook}}')" )"
  txhash="$(echo "$txjson" | tx_hash_from_json)"
  [[ -n "$txhash" ]] || { echo "$txjson" >&2; return 1; }
  sleep 2
  SWAP_TX="$txhash"
}

# On-chain cap (default 15); override only for regression against stale wasm.
MAX_PARKS_CAP="${VERIFY309_MAX_PARKS_CAP:-15}"

run_sweep_case() {
  local n="$1"
  local case_index="$2"
  local now_sec short_exp gas parks capped skipped events tx_bytes

  resolve_pair $((PAIR_INDEX_BASE + case_index))
  echo ""
  echo "==> N=$n expired bids at head (PAIR=$PAIR)"
  now_sec="$(latest_block_unix)"
  short_exp=$((now_sec + EXPIRY_LEAD_SEC))
  place_expired_bids "$n" "$short_exp"

  echo "    wait for chain time >= $short_exp"
  local waited=0
  while (( waited < 120 )); do
    now_sec="$(latest_block_unix)"
    (( now_sec >= short_exp )) && break
    sleep 2
    waited=$((waited + 2))
  done
  if (( now_sec < short_exp )); then
    bad "N=$n: chain time did not reach expires_at"
    return
  fi

  hybrid_swap_book_only "$BOOK_INPUT_RAW"
  gas="$(tx_gas_used "$SWAP_TX" || true)"
  parks="$(tx_wasm_attr "$SWAP_TX" expired_parks_used || true)"
  capped="$(tx_wasm_attr "$SWAP_TX" expired_parks_capped || true)"
  skipped="$(tx_wasm_attr "$SWAP_TX" expired_parks_skipped || true)"
  events="$(tx_wasm_event_count "$SWAP_TX" limit_order_expired_parked || true)"
  tx_bytes="$(tx_serialized_bytes "$SWAP_TX" || true)"

  local expected_parks=$(( n < MAX_PARKS_CAP ? n : MAX_PARKS_CAP ))
  local expected_skipped=$(( n > MAX_PARKS_CAP ? n - MAX_PARKS_CAP : 0 ))

  echo "    tx=$SWAP_TX gas_used=${gas:-?} parks=${parks:-?} capped=${capped:-} skipped=${skipped:-} events=${events:-?} tx_bytes=${tx_bytes:-?}"

  BENCH_ROWS+=("| $n | ${gas:-n/a} | ${events:-n/a} | ${tx_bytes:-n/a} | ${parks:-n/a} | ${skipped:-0} |")

  if [[ "$parks" == "$expected_parks" ]]; then
    ok "N=$n: expired_parks_used=$parks (expected $expected_parks)"
  else
    bad "N=$n: expired_parks_used=$parks (expected $expected_parks)"
  fi

  if (( expected_skipped > 0 )); then
    if [[ "$capped" == "true" && "$skipped" == "$expected_skipped" ]]; then
      ok "N=$n: cap attrs capped=true skipped=$skipped"
    else
      bad "N=$n: expected capped=true skipped=$expected_skipped, got capped=$capped skipped=$skipped"
    fi
  elif [[ -z "$capped" || "$capped" == "null" ]]; then
    ok "N=$n: no cap attr below MAX_EXPIRED_PARKS_PER_SWAP"
  else
    bad "N=$n: unexpected capped=$capped below cap"
  fi

  if [[ -n "$gas" ]] && (( gas < GAS_CEILING )); then
    ok "N=$n: gas_used=$gas < ceiling=$GAS_CEILING"
  elif [[ -n "$gas" ]]; then
    bad "N=$n: gas_used=$gas exceeds ceiling=$GAS_CEILING"
  else
    bad "N=$n: gas_used unavailable"
  fi

  if [[ -n "$tx_bytes" ]] && (( tx_bytes < TX_BYTES_CEILING )); then
    ok "N=$n: tx_bytes=$tx_bytes < $TX_BYTES_CEILING"
  elif [[ -n "$tx_bytes" ]]; then
    bad "N=$n: tx_bytes=$tx_bytes exceeds limit"
  fi
}

echo "==> GitLab #309 expired-park gas benchmark"
echo "    FACTORY=$FACTORY  LCD=$LCD  SWEEP=$SWEEP"
echo "    MAX_PARKS_CAP=$MAX_PARKS_CAP  GAS_CEILING=$GAS_CEILING"
echo "    PAIR_INDEX_BASE=$PAIR_INDEX_BASE (one fresh pair per sweep case)"

echo "==> Provision dev wallet CW20 (idempotent)"
bash "$REPO_ROOT/scripts/e2e-provision-dev-wallet.sh"

IFS=',' read -ra SWEEP_ARR <<< "$SWEEP"
case_index=0
for n in "${SWEEP_ARR[@]}"; do
  n="$(echo "$n" | tr -d ' ')"
  [[ -n "$n" ]] || continue
  run_sweep_case "$n" "$case_index"
  case_index=$((case_index + 1))
done

echo ""
echo "==> Benchmark table (paste into docs/limit-orders.md)"
echo "| N expired | gas_used | park events | tx bytes | parks_used | skipped |"
echo "|-----------|----------|-------------|----------|------------|---------|"
for row in "${BENCH_ROWS[@]}"; do
  echo "$row"
done

echo ""
echo "==> Summary ($PASS passed, $FAIL failed)"
for r in "${RESULTS[@]}"; do echo "    $r"; done

if (( FAIL > 0 )); then
  exit 1
fi
