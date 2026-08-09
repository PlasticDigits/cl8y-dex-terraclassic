#!/usr/bin/env bash
# Create + seed a UST1 secondary AMM pair on columbus-5 (GitLab #508 Path A).
#
# Usage:
#   DRY_RUN=1 ./scripts/add-ust1-secondary-pair.sh              # print plan + preflight
#   UST1_SEC_PAIR_LEG=vfdusd ./scripts/add-ust1-secondary-pair.sh
#   UST1_SEC_SKIP_LP=1 UST1_SEC_ALLOW_UNSEEDED=1 ./scripts/add-ust1-secondary-pair.sh
#     # create without seed — empty markets violate U4; ALLOW_UNSEEDED required
#
# Requires: host terrad, funded key with pair-creation fee + (for seed) both CW20 balances.
# Does NOT modify soft-launch gemstone defaults (invariant U6).
#
# Prefer TTY unlock; for non-interactive: export TERRAD_HOST_KEYRING_PASS (never commit).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/ust1-secondary-pair-defaults.sh
source "$SCRIPT_DIR/lib/ust1-secondary-pair-defaults.sh"
# shellcheck source=lib/terrad-host.sh
source "$SCRIPT_DIR/lib/terrad-host.sh"
# shellcheck source=lib/terrad-tx-events.sh
source "$SCRIPT_DIR/lib/terrad-tx-events.sh"
# shellcheck source=lib/lcd-smart-query.sh
source "$SCRIPT_DIR/lib/lcd-smart-query.sh"

TERRAD_HOST_KEY="$UST1_SEC_DEPLOY_KEY"
TERRAD_HOST_EXPECTED_ADDR="$UST1_SEC_DEPLOY_ADDR"
TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-columbus-5}"
TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-https://terra-classic-rpc.publicnode.com:443}"
LCD_URL="${UST1_SEC_LCD_URL:-${TERRA_LCD_URL:-https://terra-classic-lcd.publicnode.com}}"
LCD_URL="${LCD_URL%/}"

DEPLOY_DIR="$REPO_ROOT/$UST1_SEC_DEPLOY_DIR_REL"
ENV_OUT="${UST1_SEC_ENV_OUT:-$DEPLOY_DIR/addresses.env}"
TRACE_OUT="${UST1_SEC_TRACE_OUT:-$DEPLOY_DIR/deploy-trace.md}"
mkdir -p "$DEPLOY_DIR"

QUOTE_ADDR="$(ust1_sec_quote_address)"
QUOTE_SYM="$(ust1_sec_quote_symbol)"
PAIR_LABEL="$(ust1_sec_pair_label)"
UST1_ADDR="$UST1_SEC_UST1_ADDRESS"
FACTORY="$UST1_SEC_FACTORY_ADDRESS"
FEE_DISC="$UST1_SEC_FEE_DISCOUNT_ADDRESS"
SEED_A="$UST1_SEC_SEED_AMOUNT_A"
SEED_B="$UST1_SEC_SEED_AMOUNT_B"
SKIP_LP="${UST1_SEC_SKIP_LP:-0}"
ALLOW_UNSEEDED="${UST1_SEC_ALLOW_UNSEEDED:-0}"
SET_DISCOUNT="${UST1_SEC_SET_DISCOUNT_REGISTRY:-1}"
ALLOW_DISCOUNT_FAIL="${UST1_SEC_ALLOW_DISCOUNT_FAIL:-0}"

echo "=============================================="
echo "UST1 secondary AMM pair (#508)"
echo "=============================================="
echo "Pair:     $PAIR_LABEL"
echo "Factory:  $FACTORY"
echo "UST1:     $UST1_ADDR"
echo "Quote:    $QUOTE_ADDR ($QUOTE_SYM)"
echo "Seed:     $SEED_A / $SEED_B (raw)"
echo "DRY_RUN:  ${DRY_RUN:-0}"
echo "SKIP_LP:  $SKIP_LP"
echo ""

die() { echo "ERROR: $*" >&2; exit 1; }

norm_sym() { printf '%s' "$1" | tr '[:upper:]' '[:lower:]'; }

if [[ "$SKIP_LP" == "1" && "$ALLOW_UNSEEDED" != "1" ]]; then
  die "UST1_SEC_SKIP_LP=1 refused without UST1_SEC_ALLOW_UNSEEDED=1 (empty markets violate U4 / Path B)"
fi

contract_code_id() {
  local addr="$1"
  curl -sS --max-time 20 "${LCD_URL}/cosmwasm/wasm/v1/contract/${addr}" \
    | jq -r '.contract_info.code_id // empty'
}

token_info_symbol() {
  local addr="$1"
  lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD_URL" "$addr" '{"token_info":{}}')" \
    | jq -r '.symbol // empty'
}

cw20_balance() {
  local token="$1" owner="$2"
  lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD_URL" "$token" "{\"balance\":{\"address\":\"$owner\"}}")" \
    | jq -r '.balance // "0"'
}

factory_pair_addr() {
  local a="$1" b="$2"
  local q raw
  q="$(jq -nc --arg a "$a" --arg b "$b" \
    '{pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
  set +e
  raw="$(lcd_smart_query_raw "$LCD_URL" "$FACTORY" "$q" 2>/dev/null)"
  set -e
  if echo "$raw" | jq -e '.data' >/dev/null 2>&1; then
    lcd_decode_smart_data "$raw" | jq -r '.contract_addr // .pair.contract_addr // empty'
  else
    printf ''
  fi
}

pair_total_share() {
  local pair="$1"
  lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD_URL" "$pair" '{"pool":{}}')" \
    | jq -r '.total_share // "0"'
}

factory_pair_fee() {
  lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD_URL" "$FACTORY" '{"config":{}}')" \
    | jq -r '.pair_creation_fee_uluna // "0"'
}

broadcast_and_wait() {
  local label="$1"
  shift
  local out tx_hash
  echo "  → $label" >&2
  out="$(terrad_host_tx "$@")"
  tx_hash="$(printf '%s' "$out" | jq -r '.txhash // empty')"
  [[ -n "$tx_hash" ]] || {
    echo "ERROR: no txhash from: $label" >&2
    printf '%s\n' "$out" >&2
    exit 1
  }
  echo "    tx: $tx_hash" >&2
  terrad_host_wait_tx_inclusion "$tx_hash"
  printf '%s' "$tx_hash"
}

# --- Preflight (U2/U3) ---
echo "[preflight] token_info + code IDs + symbol match"
for addr_sym in "$UST1_ADDR:UST1" "$QUOTE_ADDR:$QUOTE_SYM"; do
  addr="${addr_sym%%:*}"
  sym="${addr_sym##*:}"
  got_sym="$(token_info_symbol "$addr")"
  code_id="$(contract_code_id "$addr")"
  echo "  $sym addr=$addr symbol=$got_sym code_id=$code_id"
  [[ -n "$got_sym" ]] || die "token_info failed for $addr"
  if [[ "$(norm_sym "$got_sym")" != "$(norm_sym "$sym")" ]]; then
    die "$addr on-chain symbol='$got_sym' expected '$sym' (wrong token env override?)"
  fi
  if [[ "$code_id" != "$UST1_SEC_EXPECTED_CW20_CODE_ID" ]]; then
    die "$sym code_id=$code_id expected $UST1_SEC_EXPECTED_CW20_CODE_ID (whitelist policy / U2)"
  fi
done

FEE_ULUNA="$(factory_pair_fee)"
echo "  pair_creation_fee_uluna=$FEE_ULUNA"

if [[ "${DRY_RUN:-0}" != "1" ]]; then
  DEPLOY_ADDR="$(terrad_host_key_address)" || exit 1
  [[ "$DEPLOY_ADDR" == "$UST1_SEC_DEPLOY_ADDR" ]] || die "unexpected deployer $DEPLOY_ADDR (want $UST1_SEC_DEPLOY_ADDR)"
else
  DEPLOY_ADDR="$UST1_SEC_DEPLOY_ADDR"
  echo "  [DRY_RUN] skipping key unlock; assuming deployer $DEPLOY_ADDR"
fi

BAL_A="$(cw20_balance "$UST1_ADDR" "$DEPLOY_ADDR")"
BAL_B="$(cw20_balance "$QUOTE_ADDR" "$DEPLOY_ADDR")"
echo "  deployer balances: UST1=$BAL_A $QUOTE_SYM=$BAL_B"

EXISTING="$(factory_pair_addr "$UST1_ADDR" "$QUOTE_ADDR" || true)"
if [[ -z "$EXISTING" ]]; then
  EXISTING="$(factory_pair_addr "$QUOTE_ADDR" "$UST1_ADDR" || true)"
fi
if [[ -n "$EXISTING" ]]; then
  echo "  existing factory pair: $EXISTING (total_share=$(pair_total_share "$EXISTING"))"
else
  echo "  no factory pair yet for $PAIR_LABEL"
fi

if [[ "$SKIP_LP" != "1" ]]; then
  if [[ "$BAL_A" -lt "$SEED_A" || "$BAL_B" -lt "$SEED_B" ]]; then
    if [[ "${DRY_RUN:-0}" == "1" ]]; then
      echo "  WARN: insufficient CW20 balances for seed (need $SEED_A / $SEED_B) — live Path A blocked until inventory (U4/U7)."
    else
      die "insufficient CW20 for seed (UST1=$BAL_A need $SEED_A; $QUOTE_SYM=$BAL_B need $SEED_B). Mint via /ust1 window or fund deployer, or set UST1_SEC_SKIP_LP=1 UST1_SEC_ALLOW_UNSEEDED=1 (empty markets violate U4)."
    fi
  fi
fi

if [[ "${DRY_RUN:-0}" == "1" ]]; then
  echo ""
  echo "[DRY_RUN] plan:"
  echo "  1. create_pair UST1+$QUOTE_SYM on $FACTORY (fee ${FEE_ULUNA}uluna) if missing"
  if [[ "$SET_DISCOUNT" == "1" ]]; then
    echo "  2. set_discount_registry on new pair → $FEE_DISC"
  fi
  if [[ "$SKIP_LP" != "1" ]]; then
    echo "  3. increase_allowance ×2 + provide_liquidity ($SEED_A / $SEED_B)"
  else
    echo "  3. SKIP LP (ALLOW_UNSEEDED=$ALLOW_UNSEEDED)"
  fi
  echo "  4. write $ENV_OUT + append trace $TRACE_OUT"
  echo ""
  echo "DRY_RUN complete (no txs)."
  exit 0
fi

# --- Create ---
PAIR_ADDR="$EXISTING"
CREATE_TX=""
if [[ -z "$PAIR_ADDR" ]]; then
  CREATE_MSG="$(jq -nc --arg a "$UST1_ADDR" --arg b "$QUOTE_ADDR" \
    '{create_pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
  CREATE_TX="$(broadcast_and_wait "create_pair $PAIR_LABEL" \
    wasm execute "$FACTORY" "$CREATE_MSG" --amount "${FEE_ULUNA}uluna")"
  sleep 2
  PAIR_ADDR="$(factory_pair_addr "$UST1_ADDR" "$QUOTE_ADDR")"
  [[ -n "$PAIR_ADDR" ]] || die "create_pair succeeded but factory Pair query empty (tx $CREATE_TX)"
  echo "  pair: $PAIR_ADDR"
else
  echo "  reusing pair $PAIR_ADDR"
fi

# --- Discount registry (optional; factory governance may be required post soft-launch) ---
if [[ "$SET_DISCOUNT" == "1" ]]; then
  DISC_MSG="$(jq -nc --arg p "$PAIR_ADDR" --arg r "$FEE_DISC" \
    '{set_discount_registry:{pair:$p,registry:$r}}')"
  set +e
  broadcast_and_wait "set_discount_registry" wasm execute "$FACTORY" "$DISC_MSG" >/dev/null
  DISC_ST=$?
  set -e
  if [[ "$DISC_ST" -ne 0 ]]; then
    if [[ "$ALLOW_DISCOUNT_FAIL" == "1" ]]; then
      echo "  WARN: set_discount_registry failed (UST1_SEC_ALLOW_DISCOUNT_FAIL=1); governance may need to run this." >&2
    else
      die "set_discount_registry failed — use governance multisig, or set UST1_SEC_ALLOW_DISCOUNT_FAIL=1 to continue"
    fi
  fi
fi

# --- Seed LP ---
SEED_TX=""
if [[ "$SKIP_LP" != "1" ]]; then
  SHARE="$(pair_total_share "$PAIR_ADDR")"
  if [[ "$SHARE" != "0" && -n "$SHARE" ]]; then
    echo "  pool already seeded (total_share=$SHARE); skipping provide_liquidity"
  else
    ALLOW_A="$(jq -nc --arg s "$PAIR_ADDR" --arg a "$SEED_A" \
      '{increase_allowance:{spender:$s,amount:$a,expires:{never:{}}}}')"
    ALLOW_B="$(jq -nc --arg s "$PAIR_ADDR" --arg a "$SEED_B" \
      '{increase_allowance:{spender:$s,amount:$a,expires:{never:{}}}}')"
    broadcast_and_wait "increase_allowance UST1" wasm execute "$UST1_ADDR" "$ALLOW_A" >/dev/null
    broadcast_and_wait "increase_allowance $QUOTE_SYM" wasm execute "$QUOTE_ADDR" "$ALLOW_B" >/dev/null
    PROVIDE="$(jq -nc --arg a "$UST1_ADDR" --arg b "$QUOTE_ADDR" --arg aa "$SEED_A" --arg bb "$SEED_B" \
      '{provide_liquidity:{assets:[
        {info:{token:{contract_addr:$a}},amount:$aa},
        {info:{token:{contract_addr:$b}},amount:$bb}
      ],slippage_tolerance:null,receiver:null,deadline:null}}')"
    SEED_TX="$(broadcast_and_wait "provide_liquidity $PAIR_LABEL" wasm execute "$PAIR_ADDR" "$PROVIDE")"
  fi
fi

# --- Persist ---
{
  echo "# UST1 secondary AMM pair — written by add-ust1-secondary-pair.sh $(date -u +%Y-%m-%dT%H:%MZ)"
  echo "NETWORK=mainnet"
  echo "CHAIN_ID=columbus-5"
  echo "FACTORY_ADDRESS=$FACTORY"
  echo "ROUTER_ADDRESS=$UST1_SEC_ROUTER_ADDRESS"
  echo "FEE_DISCOUNT_ADDRESS=$FEE_DISC"
  echo "UST1_TOKEN_ADDRESS=$UST1_ADDR"
  echo "QUOTE_TOKEN_ADDRESS=$QUOTE_ADDR"
  echo "QUOTE_SYMBOL=$QUOTE_SYM"
  echo "PAIR_LABEL=$PAIR_LABEL"
  echo "PAIR_ADDRESS=$PAIR_ADDR"
  echo "SEED_AMOUNT_A=$SEED_A"
  echo "SEED_AMOUNT_B=$SEED_B"
  echo "CREATE_TX=${CREATE_TX:-}"
  echo "SEED_TX=${SEED_TX:-}"
} >"$ENV_OUT"

{
  echo ""
  echo "## UST1 secondary pair — $(date -u +%Y-%m-%d) UTC (GitLab #508)"
  echo ""
  echo "| Field | Value |"
  echo "|-------|-------|"
  echo "| Pair | $PAIR_LABEL |"
  echo "| Pair address | \`$PAIR_ADDR\` |"
  echo "| Create tx | \`${CREATE_TX:-reused}\` |"
  echo "| Seed tx | \`${SEED_TX:-skipped-or-existing}\` |"
  echo "| Seed amounts (raw) | $SEED_A / $SEED_B |"
  echo "| Note | Smoke/discovery liquidity only (U4); \`/ust1\` remains primary mint/redeem (U1). |"
} >>"$TRACE_OUT"

echo ""
echo "Done."
echo "  PAIR_ADDRESS=$PAIR_ADDR"
echo "  env: $ENV_OUT"
echo "  Confirm indexer discovery + small Swap/Trade smoke; do not market AMM as mint/redeem (U1)."
