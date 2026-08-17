#!/usr/bin/env bash
# GitLab #538 — LocalTerra CreatePair inherit check (no follow-up SetDiscountRegistry).
#
# Instantiates two throwaway CW20s, create_pair, then queries pair
# GetDiscountRegistry and factory config.discount_registry. The pair must
# match the factory pointer without executing set_discount_registry.
#
# deploy-dex-local.sh still runs an idempotent per-pair Set after create; this
# script is the dedicated proof that inherit works without that belt-and-suspenders.
#
# Requires: running LocalTerra + frontend-dapp/.env.local (or VERIFY538_ENV_LOCAL).
# Exit 0 on match. Exit 2 when LocalTerra / deploy env is missing (not a code fail).
# Exit 1 on inherit mismatch or factory pointer unset.
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
TEST_ADDRESS="terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v"
CACHE="${VERIFY538_CACHE:-/tmp/cl8y-538-inherit.cache}"
INITIAL_BAL="${VERIFY538_INITIAL_BAL:-1000000000000}"

find_env_local() {
  if [[ -n "${VERIFY538_ENV_LOCAL:-}" && -f "${VERIFY538_ENV_LOCAL}" ]]; then
    printf '%s' "$VERIFY538_ENV_LOCAL"
    return 0
  fi
  if [[ -f "$REPO_ROOT/frontend-dapp/.env.local" ]]; then
    printf '%s' "$REPO_ROOT/frontend-dapp/.env.local"
    return 0
  fi
  # git worktree under .worktrees/<name> — primary checkout holds the deploy env.
  local sibling
  sibling="$(cd "$REPO_ROOT/../.." 2>/dev/null && pwd)/frontend-dapp/.env.local"
  if [[ -f "$sibling" ]]; then
    printf '%s' "$sibling"
    return 0
  fi
  return 1
}

ENV_LOCAL="$(find_env_local || true)"
if [[ -z "$ENV_LOCAL" ]]; then
  echo "localterra-create-pair-inherit: missing frontend-dapp/.env.local (run make deploy-local)." >&2
  exit 2
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
  echo "localterra-create-pair-inherit: VITE_FACTORY_ADDRESS unset." >&2
  exit 2
}
[[ -n "${VITE_FEE_DISCOUNT_ADDRESS:-}" ]] || {
  echo "localterra-create-pair-inherit: VITE_FEE_DISCOUNT_ADDRESS unset." >&2
  exit 2
}
[[ -n "${VITE_TOKEN_EMBER_ADDRESS:-}" ]] || {
  echo "localterra-create-pair-inherit: VITE_TOKEN_EMBER_ADDRESS unset." >&2
  exit 2
}

# shellcheck source=scripts/lib/localterra-host-curl.sh
source "$REPO_ROOT/scripts/lib/localterra-host-curl.sh"
# shellcheck source=scripts/lib/e2e-terrad-tx.sh
source "$REPO_ROOT/scripts/lib/e2e-terrad-tx.sh"
# shellcheck source=scripts/lib/terrad-tx-events.sh
source "$REPO_ROOT/scripts/lib/terrad-tx-events.sh"
# shellcheck source=scripts/lib/lcd-smart-query.sh
source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"

CONTAINER="$(localterra_container_id "$REPO_ROOT")"
[[ -n "$CONTAINER" ]] || {
  echo "localterra-create-pair-inherit: localterra container not running." >&2
  exit 2
}

terrad_tx() {
  e2e_terrad_tx "$CONTAINER" "$@"
}

terrad_query() {
  localterra_docker_exec "$CONTAINER" terrad query "$@" \
    --node http://127.0.0.1:26657 \
    --output json
}

txhash_from_tx_out() {
  local raw="$1"
  echo "$raw" | sed -n '/^{/,$p' | tail -1 | jq -r '.txhash // .tx_response.txhash // empty' 2>/dev/null \
    || echo "$raw" | tr '\n' ' ' | grep -oE '\{.*\}' | tail -1 | jq -r '.txhash // .tx_response.txhash // empty' 2>/dev/null \
    || true
}

contract_addr_from_tx() {
  local tx_hash="$1"
  local i json addr
  for i in $(seq 1 20); do
    json="$(terrad_query tx "$tx_hash" 2>/dev/null || true)"
    addr="$(echo "$json" | terrad_jq_contract_address_from_tx_json 2>/dev/null | head -1 || true)"
    if [[ -n "$addr" && "$addr" == terra1* ]]; then
      printf '%s' "$addr"
      return 0
    fi
    sleep 1
  done
  printf ''
  return 1
}

terrad_smart() {
  local contract="$1" msg="$2"
  terrad_query wasm contract-state smart "$contract" "$msg" | jq '.data // .'
}

LCD="${VITE_TERRA_LCD_URL:-http://localhost:1317}"
LCD="${LCD%/}"

set +e
FACTORY_CFG="$(terrad_smart "$VITE_FACTORY_ADDRESS" '{"config":{}}' 2>/dev/null)"
FACTORY_Q_ST=$?
set -e
if [[ "$FACTORY_Q_ST" -ne 0 || -z "$FACTORY_CFG" || "$FACTORY_CFG" == "null" ]]; then
  echo "localterra-create-pair-inherit: factory $VITE_FACTORY_ADDRESS not queryable." >&2
  echo "  .env.local is likely stale after a chain reset — run make deploy-local." >&2
  exit 2
fi

FACTORY_REG="$(echo "$FACTORY_CFG" | jq -r '.discount_registry // empty')"
PAIR_CREATION_FEE_ULUNA="$(echo "$FACTORY_CFG" | jq -r '.pair_creation_fee_uluna // "0"')"
CW20_CODE_ID="$(echo "$FACTORY_CFG" | jq -r '.lp_token_code_id // empty')"
if [[ -z "$CW20_CODE_ID" || "$CW20_CODE_ID" == "null" ]]; then
  CW20_CODE_ID="$(terrad_query wasm contract "$VITE_TOKEN_EMBER_ADDRESS" 2>/dev/null \
    | jq -r '.contract_info.code_id // .code_id // empty')"
fi
[[ -n "$CW20_CODE_ID" && "$CW20_CODE_ID" != "null" ]] || {
  echo "localterra-create-pair-inherit: could not resolve CW20 code id." >&2
  exit 1
}

if [[ -z "$FACTORY_REG" || "$FACTORY_REG" == "null" ]]; then
  echo "localterra-create-pair-inherit: factory config.discount_registry is null." >&2
  echo "  Run set_discount_registry_all / _batch or update_config { discount_registry } first (#538 ops)." >&2
  exit 1
fi

if [[ "$FACTORY_REG" != "$VITE_FEE_DISCOUNT_ADDRESS" ]]; then
  echo "localterra-create-pair-inherit: factory pointer $FACTORY_REG != VITE_FEE_DISCOUNT_ADDRESS $VITE_FEE_DISCOUNT_ADDRESS." >&2
  exit 1
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

instantiate_token() {
  local name="$1" sym="$2" label="$3"
  local init out tx addr
  init="$(jq -nc --arg n "$name" --arg s "$sym" --arg a "$TEST_ADDRESS" --arg amt "$INITIAL_BAL" \
    '{name:$n,symbol:$s,decimals:6,initial_balances:[{address:$a,amount:$amt}],mint:{minter:$a}}')"
  out="$(terrad_tx wasm instantiate "$CW20_CODE_ID" "$init" \
    --label "$label" --admin "$TEST_ADDRESS")"
  tx="$(txhash_from_tx_out "$out")"
  [[ -n "$tx" ]] || {
    echo "localterra-create-pair-inherit: instantiate $sym produced no txhash:" >&2
    printf '%s\n' "$out" >&2
    exit 1
  }
  addr="$(contract_addr_from_tx "$tx")"
  [[ -n "$addr" ]] || {
    echo "localterra-create-pair-inherit: instantiate $sym failed (tx $tx)." >&2
    exit 1
  }
  printf '%s' "$addr"
}

PAIR_ADDR=""
TOKEN_A=""
TOKEN_B=""
if [[ -f "$CACHE" ]]; then
  # shellcheck disable=SC1090
  source "$CACHE"
fi

if [[ -n "${INHERIT538_FACTORY:-}" && "$INHERIT538_FACTORY" == "$VITE_FACTORY_ADDRESS" \
  && -n "${INHERIT538_PAIR:-}" && -n "${INHERIT538_TOKEN_A:-}" && -n "${INHERIT538_TOKEN_B:-}" ]]; then
  EXISTING="$(factory_pair_addr "$INHERIT538_TOKEN_A" "$INHERIT538_TOKEN_B" || true)"
  if [[ "$EXISTING" == "$INHERIT538_PAIR" ]]; then
    PAIR_ADDR="$INHERIT538_PAIR"
    TOKEN_A="$INHERIT538_TOKEN_A"
    TOKEN_B="$INHERIT538_TOKEN_B"
    echo "localterra-create-pair-inherit: reusing cached pair=$PAIR_ADDR"
  fi
fi

if [[ -z "$PAIR_ADDR" ]]; then
  STAMP="$(date +%s)"
  echo "localterra-create-pair-inherit: CW20 code_id=$CW20_CODE_ID fee=${PAIR_CREATION_FEE_ULUNA}uluna factory_reg=$FACTORY_REG"
  echo "  instantiating inherit tokens..."
  TOKEN_A="$(instantiate_token "Inherit538A" "I538A" "inherit-538-a-${STAMP}")"
  echo "  token_a=$TOKEN_A"
  TOKEN_B="$(instantiate_token "Inherit538B" "I538B" "inherit-538-b-${STAMP}")"
  echo "  token_b=$TOKEN_B"

  echo "  create_pair (no SetDiscountRegistry)..."
  CREATE_MSG="$(jq -nc --arg a "$TOKEN_A" --arg b "$TOKEN_B" \
    '{create_pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
  fee_args=()
  if [[ "$PAIR_CREATION_FEE_ULUNA" != "0" && -n "$PAIR_CREATION_FEE_ULUNA" ]]; then
    fee_args=(--amount "${PAIR_CREATION_FEE_ULUNA}uluna")
  fi
  CREATE_OUT="$(terrad_tx wasm execute "$VITE_FACTORY_ADDRESS" "$CREATE_MSG" "${fee_args[@]}")"
  TX_HASH="$(txhash_from_tx_out "$CREATE_OUT")"
  [[ -n "$TX_HASH" ]] || {
    echo "localterra-create-pair-inherit: create_pair produced no txhash:" >&2
    printf '%s\n' "$CREATE_OUT" >&2
    exit 1
  }
  PAIR_ADDR="$(contract_addr_from_tx "$TX_HASH" || true)"
  if [[ -z "$PAIR_ADDR" ]]; then
    PAIR_ADDR="$(factory_pair_addr "$TOKEN_A" "$TOKEN_B" || true)"
  fi
  [[ -n "$PAIR_ADDR" ]] || {
    echo "localterra-create-pair-inherit: failed to resolve pair (tx $TX_HASH)." >&2
    exit 1
  }
  echo "  pair=$PAIR_ADDR create_tx=$TX_HASH"
  cat >"$CACHE" <<EOF
INHERIT538_FACTORY=$VITE_FACTORY_ADDRESS
INHERIT538_PAIR=$PAIR_ADDR
INHERIT538_TOKEN_A=$TOKEN_A
INHERIT538_TOKEN_B=$TOKEN_B
INHERIT538_CREATE_TX=$TX_HASH
EOF
fi

PAIR_REG="$(terrad_smart "$PAIR_ADDR" '{"get_discount_registry":{}}' | jq -r '.registry // empty')"
if [[ -z "$PAIR_REG" || "$PAIR_REG" == "null" ]]; then
  echo "localterra-create-pair-inherit: pair GetDiscountRegistry is null (did not inherit)." >&2
  echo "  factory=$FACTORY_REG pair=$PAIR_ADDR" >&2
  exit 1
fi

if [[ "$PAIR_REG" != "$FACTORY_REG" ]]; then
  echo "localterra-create-pair-inherit: pair registry $PAIR_REG != factory pointer $FACTORY_REG." >&2
  exit 1
fi

echo "localterra-create-pair-inherit: OK pair=$PAIR_ADDR registry=$PAIR_REG (no SetDiscountRegistry)"
