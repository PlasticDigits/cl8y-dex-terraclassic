#!/usr/bin/env bash
# LocalTerra fixture: instantiate UST1 + vFDUSD stand-in CW20s, create_pair, seed LP (GitLab #508).
#
# Requires: running LocalTerra + frontend-dapp/.env.local from deploy-dex-local.
# Idempotent: skips create when a factory pair already includes the stand-in UST1 address
# recorded in deployments/ust1-secondary-pair/local-addresses.env.
#
# Usage:
#   ./scripts/seed-ust1-secondary-pair-local.sh
#   UST1_SEC_LOCAL_SEED=5000000 ./scripts/seed-ust1-secondary-pair-local.sh
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
ENV_LOCAL="$REPO_ROOT/frontend-dapp/.env.local"
OUT_DIR="$REPO_ROOT/deployments/ust1-secondary-pair"
OUT_ENV="$OUT_DIR/local-addresses.env"
TEST_ADDRESS="terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v"
LIQ_RAW="${UST1_SEC_LOCAL_SEED:-1000000000}"
INITIAL_BAL="${UST1_SEC_LOCAL_INITIAL:-10000000000000}"

if [[ ! -f "$ENV_LOCAL" ]]; then
  echo "seed-ust1-secondary-pair-local: missing $ENV_LOCAL (run make deploy-local first)." >&2
  exit 1
fi

# shellcheck disable=SC1090
set -a
while IFS= read -r line || [[ -n "$line" ]]; do
  [[ "$line" =~ ^[[:space:]]*# ]] && continue
  [[ "$line" =~ ^VITE_[A-Z0-9_]+= ]] || continue
  key="${line%%=*}"
  val="${line#*=}"
  export "$key=$val"
done <"$ENV_LOCAL"
set +a

[[ -n "${VITE_FACTORY_ADDRESS:-}" ]] || {
  echo "seed-ust1-secondary-pair-local: VITE_FACTORY_ADDRESS unset." >&2
  exit 1
}
[[ -n "${VITE_TOKEN_EMBER_ADDRESS:-}" ]] || {
  echo "seed-ust1-secondary-pair-local: VITE_TOKEN_EMBER_ADDRESS unset (need CW20 code id source)." >&2
  exit 1
}

CONTAINER="$(docker compose -f "$REPO_ROOT/docker-compose.yml" ps -q localterra 2>/dev/null | head -1 || true)"
if [[ -z "$CONTAINER" ]]; then
  # Cloud Agent / non-login shells often need sg docker
  CONTAINER="$(sg docker -c "docker compose -f '$REPO_ROOT/docker-compose.yml' ps -q localterra" 2>/dev/null | head -1 || true)"
fi
[[ -n "$CONTAINER" ]] || {
  echo "seed-ust1-secondary-pair-local: localterra container not running." >&2
  exit 1
}

# shellcheck source=scripts/lib/e2e-terrad-tx.sh
source "$REPO_ROOT/scripts/lib/e2e-terrad-tx.sh"
# shellcheck source=scripts/lib/terrad-tx-events.sh
source "$REPO_ROOT/scripts/lib/terrad-tx-events.sh"
# shellcheck source=scripts/lib/lcd-smart-query.sh
source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"

terrad_tx() {
  e2e_terrad_tx "$CONTAINER" "$@"
}

terrad_query() {
  docker exec "$CONTAINER" terrad query "$@" \
    --node http://127.0.0.1:26657 \
    --output json
}

pair_addr_from_tx() {
  local tx_hash="$1"
  sleep 3
  terrad_query tx "$tx_hash" | terrad_jq_contract_address_from_tx_json | head -1
}

contract_addr_from_tx() {
  local tx_hash="$1"
  sleep 2
  terrad_query tx "$tx_hash" | terrad_jq_contract_address_from_tx_json | head -1
}

LCD="${VITE_TERRA_LCD_URL:-http://localhost:1317}"
LCD="${LCD%/}"
mkdir -p "$OUT_DIR"

# Reuse prior fixture when present + still on factory.
if [[ -f "$OUT_ENV" ]]; then
  # shellcheck disable=SC1090
  set -a
  # shellcheck source=/dev/null
  source "$OUT_ENV"
  set +a
fi

factory_pair_addr() {
  local a="$1" b="$2"
  local q raw
  q="$(jq -nc --arg a "$a" --arg b "$b" \
    '{pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
  set +e
  raw="$(lcd_smart_query_raw "$LCD" "$VITE_FACTORY_ADDRESS" "$q" 2>/dev/null)"
  set -e
  if echo "$raw" | jq -e '.data' >/dev/null 2>&1; then
    lcd_decode_smart_data "$raw" | jq -r '.contract_addr // .pair.contract_addr // empty'
  else
    printf ''
  fi
}

# Prefer terrad inside the container — host LCD + stale .env.local are common failure modes.
terrad_smart() {
  local contract="$1" msg="$2"
  terrad_query wasm contract-state smart "$contract" "$msg" | jq '.data // .'
}

set +e
FACTORY_CFG="$(terrad_smart "$VITE_FACTORY_ADDRESS" '{"config":{}}' 2>/dev/null)"
FACTORY_Q_ST=$?
set -e
if [[ "$FACTORY_Q_ST" -ne 0 || -z "$FACTORY_CFG" || "$FACTORY_CFG" == "null" ]]; then
  echo "seed-ust1-secondary-pair-local: factory $VITE_FACTORY_ADDRESS not queryable on this LocalTerra." >&2
  echo "  .env.local is likely stale after a chain reset — run make deploy-local (or setup-cloud-localterra)." >&2
  exit 2
fi
PAIR_CREATION_FEE_ULUNA="$(echo "$FACTORY_CFG" | jq -r '.pair_creation_fee_uluna // "0"')"
# Prefer factory lp_token_code_id (same mintable CW20 as local deploy whitelist).
CW20_CODE_ID="$(echo "$FACTORY_CFG" | jq -r '.lp_token_code_id // empty')"
if [[ -z "$CW20_CODE_ID" || "$CW20_CODE_ID" == "null" ]]; then
  CW20_CODE_ID="$(terrad_query wasm contract "$VITE_TOKEN_EMBER_ADDRESS" 2>/dev/null \
    | jq -r '.contract_info.code_id // .code_id // empty')"
fi
[[ -n "$CW20_CODE_ID" && "$CW20_CODE_ID" != "null" ]] || {
  echo "seed-ust1-secondary-pair-local: could not resolve CW20 code id (factory lp_token_code_id / EMBER)." >&2
  exit 1
}

instantiate_token() {
  local name="$1" sym="$2" label="$3"
  local init tx addr
  init="$(jq -nc --arg n "$name" --arg s "$sym" --arg a "$TEST_ADDRESS" --arg amt "$INITIAL_BAL" \
    '{name:$n,symbol:$s,decimals:6,initial_balances:[{address:$a,amount:$amt}],mint:{minter:$a}}')"
  tx="$(terrad_tx wasm instantiate "$CW20_CODE_ID" "$init" \
    --label "$label" --admin "$TEST_ADDRESS" | jq -r '.txhash')"
  addr="$(contract_addr_from_tx "$tx")"
  [[ -n "$addr" ]] || {
    echo "seed-ust1-secondary-pair-local: instantiate $sym failed (tx $tx)." >&2
    exit 1
  }
  printf '%s' "$addr"
}

if [[ -n "${LOCAL_UST1_TOKEN_ADDRESS:-}" && -n "${LOCAL_VFDUSD_TOKEN_ADDRESS:-}" && -n "${LOCAL_UST1_VFDUSD_PAIR_ADDRESS:-}" ]]; then
  EXISTING="$(factory_pair_addr "$LOCAL_UST1_TOKEN_ADDRESS" "$LOCAL_VFDUSD_TOKEN_ADDRESS" || true)"
  if [[ "$EXISTING" == "$LOCAL_UST1_VFDUSD_PAIR_ADDRESS" ]]; then
    SHARE="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$EXISTING" '{"pool":{}}')" | jq -r '.total_share // "0"')"
    if [[ "$SHARE" != "0" && -n "$SHARE" ]]; then
      echo "seed-ust1-secondary-pair-local: fixture already seeded pair=$EXISTING total_share=$SHARE"
      exit 0
    fi
  fi
fi

echo "seed-ust1-secondary-pair-local: CW20 code_id=$CW20_CODE_ID fee=${PAIR_CREATION_FEE_ULUNA}uluna seed=$LIQ_RAW"

if [[ -z "${LOCAL_UST1_TOKEN_ADDRESS:-}" ]]; then
  echo "  instantiating stand-in UST1..."
  LOCAL_UST1_TOKEN_ADDRESS="$(instantiate_token "UST1" "UST1" "ust1-secondary-fixture")"
  echo "  UST1=$LOCAL_UST1_TOKEN_ADDRESS"
fi
if [[ -z "${LOCAL_VFDUSD_TOKEN_ADDRESS:-}" ]]; then
  echo "  instantiating stand-in vFDUSD..."
  LOCAL_VFDUSD_TOKEN_ADDRESS="$(instantiate_token "Venus FDUSD (bridged)" "vFDUSD" "vfdusd-secondary-fixture")"
  echo "  vFDUSD=$LOCAL_VFDUSD_TOKEN_ADDRESS"
fi

PAIR_ADDR="$(factory_pair_addr "$LOCAL_UST1_TOKEN_ADDRESS" "$LOCAL_VFDUSD_TOKEN_ADDRESS" || true)"
if [[ -z "$PAIR_ADDR" ]]; then
  PAIR_ADDR="$(factory_pair_addr "$LOCAL_VFDUSD_TOKEN_ADDRESS" "$LOCAL_UST1_TOKEN_ADDRESS" || true)"
fi

if [[ -z "$PAIR_ADDR" ]]; then
  echo "  create_pair UST1/vFDUSD..."
  CREATE_MSG="$(jq -nc --arg a "$LOCAL_UST1_TOKEN_ADDRESS" --arg b "$LOCAL_VFDUSD_TOKEN_ADDRESS" \
    '{create_pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
  fee_args=()
  if [[ "$PAIR_CREATION_FEE_ULUNA" != "0" && -n "$PAIR_CREATION_FEE_ULUNA" ]]; then
    fee_args=(--amount "${PAIR_CREATION_FEE_ULUNA}uluna")
  fi
  TX_HASH="$(terrad_tx wasm execute "$VITE_FACTORY_ADDRESS" "$CREATE_MSG" "${fee_args[@]}" | jq -r '.txhash')"
  PAIR_ADDR="$(pair_addr_from_tx "$TX_HASH")"
  if [[ -z "$PAIR_ADDR" ]]; then
    PAIR_ADDR="$(factory_pair_addr "$LOCAL_UST1_TOKEN_ADDRESS" "$LOCAL_VFDUSD_TOKEN_ADDRESS" || true)"
  fi
  [[ -n "$PAIR_ADDR" ]] || {
    echo "seed-ust1-secondary-pair-local: failed to resolve pair (tx $TX_HASH)." >&2
    exit 1
  }
  echo "  pair=$PAIR_ADDR create_tx=$TX_HASH"
else
  echo "  existing pair=$PAIR_ADDR"
fi

SHARE="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$PAIR_ADDR" '{"pool":{}}')" | jq -r '.total_share // "0"')"
if [[ "$SHARE" == "0" || -z "$SHARE" ]]; then
  echo "  seeding liquidity $LIQ_RAW / $LIQ_RAW..."
  terrad_tx wasm execute "$LOCAL_UST1_TOKEN_ADDRESS" \
    "$(jq -nc --arg s "$PAIR_ADDR" --arg a "$LIQ_RAW" '{increase_allowance:{spender:$s,amount:$a,expires:{never:{}}}}')" >/dev/null
  sleep 2
  terrad_tx wasm execute "$LOCAL_VFDUSD_TOKEN_ADDRESS" \
    "$(jq -nc --arg s "$PAIR_ADDR" --arg a "$LIQ_RAW" '{increase_allowance:{spender:$s,amount:$a,expires:{never:{}}}}')" >/dev/null
  sleep 2
  PROVIDE="$(jq -nc --arg a "$LOCAL_UST1_TOKEN_ADDRESS" --arg b "$LOCAL_VFDUSD_TOKEN_ADDRESS" --arg aa "$LIQ_RAW" --arg bb "$LIQ_RAW" \
    '{provide_liquidity:{assets:[
      {info:{token:{contract_addr:$a}},amount:$aa},
      {info:{token:{contract_addr:$b}},amount:$bb}
    ],slippage_tolerance:null,receiver:null,deadline:null}}')"
  terrad_tx wasm execute "$PAIR_ADDR" "$PROVIDE" >/dev/null
  SHARE="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$PAIR_ADDR" '{"pool":{}}')" | jq -r '.total_share // "0"')"
fi

if [[ -n "${VITE_FEE_DISCOUNT_ADDRESS:-}" ]]; then
  DISC="$(jq -nc --arg p "$PAIR_ADDR" --arg r "$VITE_FEE_DISCOUNT_ADDRESS" \
    '{set_discount_registry:{pair:$p,registry:$r}}')"
  terrad_tx wasm execute "$VITE_FACTORY_ADDRESS" "$DISC" >/dev/null || true
fi

{
  echo "# LocalTerra UST1 secondary pair fixture — $(date -u +%Y-%m-%dT%H:%MZ) (GitLab #508)"
  echo "LOCAL_UST1_TOKEN_ADDRESS=$LOCAL_UST1_TOKEN_ADDRESS"
  echo "LOCAL_VFDUSD_TOKEN_ADDRESS=$LOCAL_VFDUSD_TOKEN_ADDRESS"
  echo "LOCAL_UST1_VFDUSD_PAIR_ADDRESS=$PAIR_ADDR"
  echo "LOCAL_SEED_AMOUNT_RAW=$LIQ_RAW"
  echo "LOCAL_PAIR_TOTAL_SHARE=$SHARE"
  echo "FACTORY_ADDRESS=$VITE_FACTORY_ADDRESS"
} >"$OUT_ENV"

# Append Vite hints for local QA (do not overwrite production Coolify secrets).
if ! grep -q '^VITE_UST1_TOKEN_ADDRESS=' "$ENV_LOCAL" 2>/dev/null; then
  {
    echo ""
    echo "# GitLab #508 LocalTerra UST1 secondary fixture (stand-ins; not columbus-5 anchors)"
    echo "VITE_UST1_TOKEN_ADDRESS=$LOCAL_UST1_TOKEN_ADDRESS"
    echo "VITE_VFDUSD_TOKEN_ADDRESS=$LOCAL_VFDUSD_TOKEN_ADDRESS"
  } >>"$ENV_LOCAL"
fi

echo "seed-ust1-secondary-pair-local: OK pair=$PAIR_ADDR total_share=$SHARE"
echo "  wrote $OUT_ENV"
echo "  Reminder: AMM is secondary; /ust1 window is primary mint/redeem when enabled (U1)."
