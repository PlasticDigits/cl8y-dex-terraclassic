#!/usr/bin/env bash
# Rebalance UST1/cUSTC and cLUNC/cUSTC to indexer oracles via mint + pool-only
# swap, then holder-burn leftover wrap on the ops wallet. Does NOT provide LP.
#
# Peg:
#   1 UST1 = $1  →  UST1/cUSTC human cUSTC-per-UST1 = 1 / USTC_USD
#   cLUNC/cUSTC  = LUNC_USD / USTC_USD
#
# This is not the LP deepen scripts (rebalance-mint-ust1-lp.sh,
# rebalance-mint-clunc-custc-lp.sh) and not mint-clunc-custc-lp.sh (keep premium).
# Swapping cLUNC/cUSTC onto the oracle sells a cLUNC premium (intended once
# the premium has exceeded the target).
#
# Atomicity:
#   Mint uses different signers (2-of-3 / primary minter) so it cannot share a
#   tx with the swaps. Mint does not move AMM prices. Both pool swaps go in
#   ONE admin tx (multiple MsgExecuteContract) so searchers cannot trade
#   between the two pegs. Leftover burn is after (does not move AMM prices).
#   If a pair is already on peg it is omitted from that tx.
#
# Flow:
#   1. Fetch indexer LUNC/USTC oracles; cross-check live CEX (same as cLUNC LP).
#   2. Mint inventory as needed (UST1/cUSTC via DEX 2-of-3; cLUNC via primary
#      minter or wrap).
#   3. Re-query reserves, plan both offers, broadcast one admin tx with every
#      pool-only CW20 Send+Swap.
#   4. Refuse to continue if either selected pool is still outside 0.1%.
#   5. Holder-burn leftover UST1 / cUSTC / cLUNC on admin. Never burns LP, USTR,
#      native uluna, or CMM holdings.
#
# Usage:
#   DRY_RUN=1 ./scripts/rebalance-oracle-mint-swap-burn.sh
#   ORACLE_RB_YES=1 ./scripts/rebalance-oracle-mint-swap-burn.sh
#   ORACLE_RB_PAIRS=ust1-custc   # or clunc-custc / both (default)
#   ORACLE_RB_BURN_ONLY=1 ORACLE_RB_YES=1 ./scripts/rebalance-oracle-mint-swap-burn.sh
#
# Unlock once (non-interactive):
#   read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
#
# Never commit TERRAD_HOST_KEYRING_PASS.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/oracle-rebalance-defaults.sh
source "$SCRIPT_DIR/lib/oracle-rebalance-defaults.sh"
# shellcheck source=lib/terrad-host.sh
source "$SCRIPT_DIR/lib/terrad-host.sh"

TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-columbus-5}"
TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-https://terra-classic-rpc.publicnode.com:443}"
LCD_URL="${ORACLE_RB_LCD_URL%/}"
INDEXER="${ORACLE_RB_INDEXER%/}"
UST1_MATH="$SCRIPT_DIR/lib/ust1-lp-rebalance-math.py"
CLUNC_MATH="$SCRIPT_DIR/lib/clunc-custc-lp-math.py"

FACTORY="${UST1_LP_FACTORY}"
UST1="${UST1_LP_UST1}"
CUSTC="${UST1_LP_CUSTC}"
CLUNC="${CLUNC_LP_CLUNC}"
PAIR_UST1="${UST1_LP_PAIR_CUSTC}"
PAIR_CLUNC="${CLUNC_LP_PAIR}"
LP_CLUNC_PIN="${CLUNC_LP_LP_TOKEN}"
LP_UST1=""
LP_CLUNC=""
TREASURY="${UST1_LP_TREASURY}"
WRAP_MAPPER="${CLUNC_LP_WRAP_MAPPER}"

MSIG_KEY="${UST1_LP_MSIG_KEY}"
SIGNER1="${UST1_LP_SIGNER1}"
SIGNER2="${UST1_LP_SIGNER2}"
MSIG_ADDR="${UST1_LP_MSIG_ADDR}"
MINTER_KEY="${CLUNC_LP_MINTER_KEY}"
MINTER_ADDR="${CLUNC_LP_MINTER_ADDR}"
ADMIN_KEY="${UST1_LP_ADMIN_KEY}"
ADMIN_ADDR="${UST1_LP_ADMIN_ADDR}"
MINT_MODE="${CLUNC_LP_MINT_MODE}"
TOLERANCE="${ORACLE_RB_PRICE_TOLERANCE}"
CROSS_TOL="${CLUNC_LP_CROSS_CHECK_TOLERANCE}"
MAX_AGE="${CLUNC_LP_ORACLE_MAX_AGE_SEC}"
SWAP_MAX_SPREAD="${ORACLE_RB_SWAP_MAX_SPREAD}"
SWAP_GAS="${ORACLE_RB_SWAP_GAS}"
LCD_TIMEOUT="${ORACLE_RB_LCD_TIMEOUT}"
GAS_RESERVE_ULUNA="${CLUNC_LP_GAS_RESERVE_ULUNA:-2000000000}"
PAIRS="${ORACLE_RB_PAIRS}"
HTTP_UA="cl8y-dex-ops/oracle-rebalance (+https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic)"

die() { echo "ERROR: $*" >&2; exit 1; }

usage() {
  sed -n '2,42p' "$0" | sed 's/^# \{0,1\}//'
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

DO_UST1=0
DO_CLUNC=0
case "$PAIRS" in
  both)
    DO_UST1=1
    DO_CLUNC=1
    ;;
  ust1-custc|ust1|custc-ust1)
    DO_UST1=1
    ;;
  clunc-custc|clunc)
    DO_CLUNC=1
    ;;
  *)
    die "ORACLE_RB_PAIRS must be both, ust1-custc, or clunc-custc (got $PAIRS)"
    ;;
esac

case "$MINT_MODE" in
  minter|wrap) ;;
  *) die "CLUNC_LP_MINT_MODE must be minter or wrap (got $MINT_MODE)" ;;
esac

lcd_b64() {
  if [[ "$(uname)" == Darwin ]]; then
    printf '%s' "$1" | base64 | tr -d '\n'
  else
    printf '%s' "$1" | base64 -w0
  fi
}

lcd_smart() {
  local contract="$1" msg="$2" raw
  raw="$(curl -sS --connect-timeout 10 --max-time "$LCD_TIMEOUT" -H "User-Agent: $HTTP_UA" \
    "${LCD_URL}/cosmwasm/wasm/v1/contract/${contract}/smart/$(lcd_b64 "$msg")")"
  jq -e '(.data != null) and ((.code // 0) == 0)' >/dev/null <<<"$raw" \
    || die "LCD smart query failed for $contract: $msg"
  if [[ "$(jq -r '.data | type' <<<"$raw")" == "string" ]]; then
    jq -r '.data | @base64d | fromjson' <<<"$raw"
  else
    jq '.data' <<<"$raw"
  fi
}

lcd_bank() {
  local addr="$1" denom="$2"
  curl -sS --connect-timeout 10 --max-time "$LCD_TIMEOUT" -H "User-Agent: $HTTP_UA" \
    "${LCD_URL}/cosmos/bank/v1beta1/balances/${addr}/by_denom?denom=${denom}" \
    | jq -r '.balance.amount // "0"'
}

cw20_balance() {
  lcd_smart "$1" "$(jq -nc --arg a "$2" '{balance:{address:$a}}')" | jq -r '.balance // "0"'
}

token_decimals() {
  lcd_smart "$1" '{"token_info":{}}' | jq -r '.decimals'
}

token_symbol() {
  lcd_smart "$1" '{"token_info":{}}' | jq -r '.symbol'
}

is_minter() {
  local token="$1" who="$2" primary extras
  primary="$(lcd_smart "$token" '{"minter":{}}' | jq -r '.minter // empty')"
  extras="$(lcd_smart "$token" '{"minters":{}}' | jq -r '[.minters[]?] | join(" ")' 2>/dev/null || echo "")"
  [[ "$primary" == "$who" || " $extras " == *" $who "* ]]
}

pool_json() { lcd_smart "$1" '{"pool":{}}'; }

asset_amount_for() {
  local pool="$1" token="$2"
  jq -r --arg t "$token" '
    .assets[]
    | select(.info.token.contract_addr == $t)
    | .amount
  ' <<<"$pool"
}

prompt_keyring_pass() {
  if [[ -n "${TERRAD_HOST_KEYRING_PASS:-}" ]]; then
    return 0
  fi
  if [[ ! -t 0 ]]; then
    die "TERRAD_HOST_KEYRING_PASS unset and stdin is not a TTY. Unlock once:
  read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS"
  fi
  read -rs -p "terrad keyring passphrase: " TERRAD_HOST_KEYRING_PASS
  echo
  export TERRAD_HOST_KEYRING_PASS
  [[ -n "$TERRAD_HOST_KEYRING_PASS" ]] || die "empty passphrase"
}

broadcast_from() {
  local key="$1" expected="$2" label="$3"
  shift 3
  local out tx_hash
  echo "  → $label" >&2
  TERRAD_HOST_KEY="$key"
  TERRAD_HOST_EXPECTED_ADDR="$expected"
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

broadcast_admin() {
  local label="$1"
  shift
  broadcast_from "$ADMIN_KEY" "$ADMIN_ADDR" "$label" "$@"
}

broadcast_minter() {
  local label="$1"
  shift
  broadcast_from "$MINTER_KEY" "$MINTER_ADDR" "$label" "$@"
}

broadcast_msig() {
  local label="$1" contract="$2" msg="$3"
  local workdir unsigned sig1 sig2 signed out tx_hash code
  echo "  → msig $label" >&2
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "    [DRY_RUN] wasm execute $contract $msg" >&2
    echo "DRY_RUN_TX"
    return 0
  fi
  workdir="$(mktemp -d "${TMPDIR:-/tmp}/oracle-rb-msig.XXXXXX")"
  unsigned="$workdir/unsigned.json"
  sig1="$workdir/sig1.json"
  sig2="$workdir/sig2.json"
  signed="$workdir/signed.json"
  # shellcheck disable=SC2046
  terrad_host_exec tx wasm execute "$contract" "$msg" \
    --from "$MSIG_KEY" \
    --generate-only \
    $(terrad_host_common_flags) \
    $(terrad_host_gas_flags) \
    $(terrad_host_fee_flags) \
    --output json >"$unsigned"
  jq -e '.body.messages | length > 0' "$unsigned" >/dev/null \
    || die "unsigned mint tx invalid ($label)"
  # shellcheck disable=SC2046
  terrad_host_exec tx sign "$unsigned" \
    --from "$SIGNER1" --multisig "$MSIG_KEY" --sign-mode amino-json \
    $(terrad_host_common_flags) --output json >"$sig1"
  # shellcheck disable=SC2046
  terrad_host_exec tx sign "$unsigned" \
    --from "$SIGNER2" --multisig "$MSIG_KEY" --sign-mode amino-json \
    $(terrad_host_common_flags) --output json >"$sig2"
  # shellcheck disable=SC2046
  terrad_host_exec tx multisign "$unsigned" "$MSIG_KEY" "$sig1" "$sig2" \
    $(terrad_host_common_flags) --output json >"$signed"
  # shellcheck disable=SC2046
  out="$(terrad_host_exec tx broadcast "$signed" \
    $(terrad_host_common_flags) \
    --broadcast-mode "${TERRAD_HOST_BROADCAST_MODE:-sync}" \
    -y --output json)"
  rm -rf "$workdir"
  tx_hash="$(printf '%s' "$out" | jq -r '.txhash // empty')"
  code="$(printf '%s' "$out" | jq -r '.code // 0')"
  [[ -n "$tx_hash" && "$code" == "0" ]] || {
    echo "ERROR: msig broadcast failed ($label)" >&2
    printf '%s\n' "$out" >&2
    exit 1
  }
  echo "    tx: $tx_hash" >&2
  terrad_host_wait_tx_inclusion "$tx_hash"
  printf '%s' "$tx_hash"
}

mint_msig() {
  local sym="$1" token="$2" amount="$3"
  if [[ "$amount" == "0" ]]; then
    echo "  skip mint $sym (admin already funded)"
    return 0
  fi
  local msg
  msg="$(jq -nc --arg r "$ADMIN_ADDR" --arg a "$amount" '{mint:{recipient:$r,amount:$a}}')"
  broadcast_msig "mint $sym $amount → $ADMIN_ADDR" "$token" "$msg" >/dev/null
}

mint_primary() {
  local sym="$1" token="$2" amount="$3"
  if [[ "$amount" == "0" ]]; then
    echo "  skip mint $sym (already funded)"
    return 0
  fi
  local msg
  msg="$(jq -nc --arg r "$ADMIN_ADDR" --arg a "$amount" '{mint:{recipient:$r,amount:$a}}')"
  if [[ "$MINTER_ADDR" == "$ADMIN_ADDR" ]]; then
    broadcast_admin "mint $sym $amount → $ADMIN_ADDR" wasm execute "$token" "$msg" >/dev/null
  else
    broadcast_minter "mint $sym $amount → $ADMIN_ADDR" wasm execute "$token" "$msg" >/dev/null
  fi
}

wrap_native() {
  local denom="$1" amount="$2" label="$3"
  if [[ "$amount" == "0" ]]; then
    echo "  skip wrap $denom (already funded)"
    return 0
  fi
  broadcast_admin "wrap $label $amount $denom" \
    wasm execute "$TREASURY" '{"wrap_deposit":{}}' --amount "${amount}${denom}" >/dev/null
}

cw20_swap_msg() {
  local pair="$1" amount="$2"
  local hook b64
  hook="$(jq -nc --arg s "$SWAP_MAX_SPREAD" \
    '{swap:{belief_price:null,max_spread:$s,to:null,deadline:null,hybrid:null,trader:null}}')"
  if [[ "$(uname)" == Darwin ]]; then
    b64="$(printf '%s' "$hook" | base64 | tr -d '\n')"
  else
    b64="$(printf '%s' "$hook" | base64 -w0)"
  fi
  jq -nc --arg c "$pair" --arg a "$amount" --arg m "$b64" \
    '{send:{contract:$c,amount:$a,msg:$m}}'
}

# One generate-only wasm execute (admin). Caller merges messages into a single tx.
generate_admin_unsigned() {
  local contract="$1" msg="$2" outfile="$3"
  local saved_gas="${TERRAD_HOST_GAS:-}"
  TERRAD_HOST_KEY="$ADMIN_KEY"
  TERRAD_HOST_EXPECTED_ADDR="$ADMIN_ADDR"
  TERRAD_HOST_GAS="$SWAP_GAS"
  # shellcheck disable=SC2046
  terrad_host_exec tx wasm execute "$contract" "$msg" \
    --from "$ADMIN_KEY" \
    --generate-only \
    $(terrad_host_common_flags) \
    $(terrad_host_gas_flags) \
    $(terrad_host_fee_flags) \
    --output json >"$outfile"
  if [[ -n "$saved_gas" ]]; then
    TERRAD_HOST_GAS="$saved_gas"
  else
    unset TERRAD_HOST_GAS
  fi
  jq -e '.body.messages | length == 1' "$outfile" >/dev/null \
    || die "generate-only produced no execute msg for $contract"
}

merge_unsigned_txs() {
  local out="$1"
  shift
  python3 - "$out" "$@" <<'PY'
import json, sys
out = sys.argv[1]
paths = sys.argv[2:]
if not paths:
    raise SystemExit("merge_unsigned_txs: no inputs")
txs = []
for path in paths:
    with open(path, encoding="utf-8") as fh:
        txs.append(json.load(fh))
merged = txs[0]
for extra in txs[1:]:
    merged["body"]["messages"].extend(extra["body"]["messages"])
with open(out, "w", encoding="utf-8") as fh:
    json.dump(merged, fh)
print(len(merged["body"]["messages"]))
PY
}

broadcast_admin_unsigned() {
  local label="$1" unsigned="$2"
  local workdir signed out tx_hash code
  echo "  → $label" >&2
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "    [DRY_RUN] atomic wasm executes:" >&2
    jq -c '.body.messages[] | {type:."@type", contract}' "$unsigned" >&2
    echo "DRY_RUN_TX"
    return 0
  fi
  workdir="$(mktemp -d "${TMPDIR:-/tmp}/oracle-rb-atomic.XXXXXX")"
  signed="$workdir/signed.json"
  TERRAD_HOST_KEY="$ADMIN_KEY"
  TERRAD_HOST_EXPECTED_ADDR="$ADMIN_ADDR"
  # shellcheck disable=SC2046
  terrad_host_exec tx sign "$unsigned" \
    --from "$ADMIN_KEY" \
    $(terrad_host_common_flags) \
    --output json >"$signed"
  # shellcheck disable=SC2046
  out="$(terrad_host_exec tx broadcast "$signed" \
    $(terrad_host_common_flags) \
    --broadcast-mode "${TERRAD_HOST_BROADCAST_MODE:-sync}" \
    -y --output json)"
  rm -rf "$workdir"
  tx_hash="$(printf '%s' "$out" | jq -r '.txhash // empty')"
  code="$(printf '%s' "$out" | jq -r '.code // 0')"
  [[ -n "$tx_hash" && "$code" == "0" ]] || {
    echo "ERROR: atomic swap broadcast failed ($label)" >&2
    printf '%s\n' "$out" >&2
    exit 1
  }
  echo "    tx: $tx_hash" >&2
  terrad_host_wait_tx_inclusion "$tx_hash"
  printf '%s' "$tx_hash"
}

assert_not_protected_token() {
  local token="$1" what="$2"
  [[ "$token" == "$LP_CLUNC" || "$token" == "$LP_CLUNC_PIN" || "$token" == "${LP_UST1:-}" ]] \
    && die "refusing to $what LP token $token"
  [[ "$token" == "$TREASURY" ]] && die "refusing to $what CMM treasury $token"
  [[ "$token" == "$PAIR_UST1" || "$token" == "$PAIR_CLUNC" ]] && die "refusing to $what pair $token"
}

burn_if_any() {
  local sym="$1" token="$2" amount="$3"
  assert_not_protected_token "$token" "burn"
  if [[ "$amount" == "0" ]]; then
    echo "  skip burn $sym (balance 0)"
    return 0
  fi
  local msg
  msg="$(jq -nc --arg a "$amount" '{burn:{amount:$a}}')"
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "  [DRY_RUN] burn $sym $amount on $token"
    return 0
  fi
  broadcast_admin "burn $sym $amount" wasm execute "$token" "$msg" >/dev/null
}

rel_err_ok() {
  python3 - "$1" "$2" "$TOLERANCE" <<'PY'
import sys
from decimal import Decimal
cur, tgt, tol = map(Decimal, sys.argv[1:])
sys.exit(0 if abs(cur - tgt) / tgt <= tol else 1)
PY
}

human_px() {
  python3 - "$1" "$2" "$3" "$4" <<'PY'
import sys
from decimal import Decimal
r0, r1, d0, d1 = sys.argv[1:]
print((Decimal(r1) / (Decimal(10) ** int(d1))) / (Decimal(r0) / (Decimal(10) ** int(d0))))
PY
}

fetch_oracles() {
  python3 "$CLUNC_MATH" --fetch-oracles <<EOF
{
  "mode": "fetch-oracles",
  "indexer": "$INDEXER",
  "max_age_sec": $MAX_AGE,
  "cross_tolerance": "$CROSS_TOL",
  "skip_cross_check": $([[ "${ORACLE_RB_SKIP_CROSS_CHECK:-${CLUNC_LP_SKIP_CROSS_CHECK:-0}}" == "1" ]] && echo true || echo false),
  "clunc_addr": "$CLUNC",
  "custc_addr": "$CUSTC"
}
EOF
}

fetch_oracles_ok() {
  local i json errs tries="${ORACLE_RB_ORACLE_RETRIES:-${CLUNC_LP_ORACLE_RETRIES:-4}}"
  for ((i = 1; i <= tries; i++)); do
    json="$(fetch_oracles)" || true
    if jq -e '.ok == true and .lunc_usd != null and .ustc_usd != null' >/dev/null 2>&1 <<<"$json"; then
      printf '%s' "$json"
      return 0
    fi
    errs="$(jq -c '.errors // []' <<<"$json" 2>/dev/null || echo "$json")"
    echo "  oracle fetch attempt $i/$tries failed: $errs" >&2
    if [[ "$i" -lt "$tries" ]]; then
      sleep $((i * 3))
    fi
  done
  echo "$json" >&2
  return 1
}

plan_ust1() {
  python3 "$UST1_MATH" <<EOF
{
  "ustc_usd": "$USTC_USD",
  "usd_custc": "0",
  "usd_ustr": "0",
  "skip_swap": false,
  "tolerance": "$TOLERANCE",
  "fee_bps": $FEE_UST1,
  "buffer_bps": 0,
  "dec_ust1": $DEC_UST1,
  "dec_custc": $DEC_CUSTC,
  "dec_ustr": 18,
  "custc_r0": "$R_UST1",
  "custc_r1": "$R_UST1_Q",
  "ustr_r0": "0",
  "ustr_r1": "0",
  "bal_ust1": "$BAL_UST1",
  "bal_custc": "$BAL_CUSTC",
  "bal_ustr": "0"
}
EOF
}

plan_clunc() {
  python3 "$CLUNC_MATH" <<EOF
{
  "mode": "plan-step",
  "lunc_usd": "$LUNC_USD",
  "ustc_usd": "$USTC_USD",
  "target_usd": "0",
  "force_add_usd": "0",
  "skip_swap": false,
  "tolerance": "$TOLERANCE",
  "fee_bps": $FEE_CLUNC,
  "buffer_bps": 0,
  "fee_wrap_bps": $FEE_WRAP_BPS,
  "dec_clunc": $DEC_CLUNC,
  "dec_custc": $DEC_CUSTC,
  "r0": "$R_CLUNC",
  "r1": "$R_CLUNC_Q",
  "bal_clunc": "$BAL_CLUNC",
  "bal_custc": "$BAL_CUSTC"
}
EOF
}

refresh_ust1_pool() {
  POOL_U="$(pool_json "$PAIR_UST1")"
  R_UST1="$(asset_amount_for "$POOL_U" "$UST1")"
  R_UST1_Q="$(asset_amount_for "$POOL_U" "$CUSTC")"
  [[ -n "$R_UST1" && -n "$R_UST1_Q" && "$R_UST1" != "0" && "$R_UST1_Q" != "0" ]] \
    || die "UST1/cUSTC pool empty or unreadable"
  PX_UST1="$(human_px "$R_UST1" "$R_UST1_Q" "$DEC_UST1" "$DEC_CUSTC")"
}

refresh_clunc_pool() {
  POOL_C="$(pool_json "$PAIR_CLUNC")"
  R_CLUNC="$(asset_amount_for "$POOL_C" "$CLUNC")"
  R_CLUNC_Q="$(asset_amount_for "$POOL_C" "$CUSTC")"
  [[ -n "$R_CLUNC" && -n "$R_CLUNC_Q" && "$R_CLUNC" != "0" && "$R_CLUNC_Q" != "0" ]] \
    || die "cLUNC/cUSTC pool empty or unreadable"
  PX_CLUNC="$(human_px "$R_CLUNC" "$R_CLUNC_Q" "$DEC_CLUNC" "$DEC_CUSTC")"
}

refresh_balances() {
  BAL_UST1="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
  BAL_CUSTC="$(cw20_balance "$CUSTC" "$ADMIN_ADDR")"
  BAL_CLUNC="$(cw20_balance "$CLUNC" "$ADMIN_ADDR")"
}

fund_ust1() {
  local amount="$1" have need
  if [[ "$amount" == "0" ]]; then
    return 0
  fi
  have="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
  if python3 -c "import sys; sys.exit(0 if int('$have') < int('$amount') else 1)"; then
    need="$(python3 -c "print(int('$amount') - int('$have'))")"
    echo "  topping up UST1 $need (have $have) via 2-of-3"
    mint_msig UST1 "$UST1" "$need"
  fi
}

fund_clunc() {
  local amount="$1" have need native
  if [[ "$amount" == "0" ]]; then
    return 0
  fi
  have="$(cw20_balance "$CLUNC" "$ADMIN_ADDR")"
  if python3 -c "import sys; sys.exit(0 if int('$have') < int('$amount') else 1)"; then
    need="$(python3 -c "print(int('$amount') - int('$have'))")"
    echo "  topping up cLUNC $need (have $have)"
    if [[ "$MINT_MODE" == "wrap" ]]; then
      native="$(python3 -c "print((int('$need') * 10000 + (10000 - $FEE_WRAP_BPS) - 1) // (10000 - $FEE_WRAP_BPS))")"
      wrap_native uluna "$native" cLUNC
    else
      mint_primary cLUNC "$CLUNC" "$need"
    fi
  fi
}

fund_custc() {
  local amount="$1" have need native
  if [[ "$amount" == "0" ]]; then
    return 0
  fi
  have="$(cw20_balance "$CUSTC" "$ADMIN_ADDR")"
  if python3 -c "import sys; sys.exit(0 if int('$have') < int('$amount') else 1)"; then
    need="$(python3 -c "print(int('$amount') - int('$have'))")"
    echo "  topping up cUSTC $need (have $have)"
    if [[ "$DO_UST1" == "1" ]]; then
      mint_msig cUSTC "$CUSTC" "$need"
    elif [[ "$MINT_MODE" == "wrap" ]]; then
      native="$(python3 -c "print((int('$need') * 10000 + (10000 - $FEE_WRAP_BPS) - 1) // (10000 - $FEE_WRAP_BPS))")"
      wrap_native uusd "$native" cUSTC
    else
      mint_primary cUSTC "$CUSTC" "$need"
    fi
  fi
}

# Build ATOMIC_SWAPS JSON: [{offer,amount,cw20,pair,label,projected,rel}].
# Independent offers — if both legs sell cUSTC, mint the SUM (msgs cannot
# spend the other swap's proceeds until after this tx).
plan_atomic_swaps() {
  local live token amt cw20 pair label
  ATOMIC_SWAPS='[]'
  NEED_UST1=0
  NEED_CUSTC=0
  NEED_CLUNC=0
  if [[ "$DO_UST1" == "1" ]]; then
    refresh_ust1_pool
    refresh_balances
    live="$(plan_ust1)"
    if [[ "$(jq -r '.swap.needed' <<<"$live")" == "true" ]]; then
      token="$(jq -r '.swap.offer_token // empty' <<<"$live")"
      amt="$(jq -r '.swap.offer_amount' <<<"$live")"
      [[ -n "$token" && "$amt" != "0" ]] \
        || die "UST1/cUSTC price $PX_UST1 off peg but planner found no swap"
      case "$token" in
        ust1) cw20="$UST1"; NEED_UST1="$(python3 -c "print(int('$NEED_UST1') + int('$amt'))")" ;;
        custc) cw20="$CUSTC"; NEED_CUSTC="$(python3 -c "print(int('$NEED_CUSTC') + int('$amt'))")" ;;
        *) die "unknown UST1/cUSTC offer token $token" ;;
      esac
      pair="$PAIR_UST1"
      label="UST1/cUSTC sell $token $amt"
      echo "  plan $label  live $R_UST1 / $R_UST1_Q  → $(jq -r '.swap.projected_price' <<<"$live")"
      ATOMIC_SWAPS="$(jq -c --arg o "$token" --arg a "$amt" --arg c "$cw20" --arg p "$pair" \
        --arg l "$label" --arg px "$(jq -r '.swap.projected_price' <<<"$live")" \
        --arg rel "$(jq -r '.swap.rel_error' <<<"$live")" \
        '. + [{offer:$o,amount:$a,cw20:$c,pair:$p,label:$l,projected:$px,rel:$rel}]' <<<"$ATOMIC_SWAPS")"
    else
      echo "  UST1/cUSTC already within $TOLERANCE at $PX_UST1"
    fi
  fi
  if [[ "$DO_CLUNC" == "1" ]]; then
    refresh_clunc_pool
    refresh_balances
    live="$(plan_clunc)"
    if [[ "$(jq -r '.swap.needed' <<<"$live")" == "true" ]]; then
      token="$(jq -r '.swap.offer_token // empty' <<<"$live")"
      amt="$(jq -r '.swap.offer_amount' <<<"$live")"
      [[ -n "$token" && "$amt" != "0" ]] \
        || die "cLUNC/cUSTC price $PX_CLUNC off peg but planner found no swap"
      case "$token" in
        clunc) cw20="$CLUNC"; NEED_CLUNC="$(python3 -c "print(int('$NEED_CLUNC') + int('$amt'))")" ;;
        custc) cw20="$CUSTC"; NEED_CUSTC="$(python3 -c "print(int('$NEED_CUSTC') + int('$amt'))")" ;;
        *) die "unknown cLUNC/cUSTC offer token $token" ;;
      esac
      pair="$PAIR_CLUNC"
      label="cLUNC/cUSTC sell $token $amt"
      echo "  plan $label  live $R_CLUNC / $R_CLUNC_Q  → $(jq -r '.swap.projected_price' <<<"$live")"
      ATOMIC_SWAPS="$(jq -c --arg o "$token" --arg a "$amt" --arg c "$cw20" --arg p "$pair" \
        --arg l "$label" --arg px "$(jq -r '.swap.projected_price' <<<"$live")" \
        --arg rel "$(jq -r '.swap.rel_error' <<<"$live")" \
        '. + [{offer:$o,amount:$a,cw20:$c,pair:$p,label:$l,projected:$px,rel:$rel}]' <<<"$ATOMIC_SWAPS")"
    else
      echo "  cLUNC/cUSTC already within $TOLERANCE at $PX_CLUNC"
    fi
  fi
}

fund_atomic_inventory() {
  echo "  need UST1=$NEED_UST1 cUSTC=$NEED_CUSTC cLUNC=$NEED_CLUNC"
  fund_ust1 "$NEED_UST1"
  fund_custc "$NEED_CUSTC"
  fund_clunc "$NEED_CLUNC"
}

execute_atomic_swaps() {
  local n workdir i contract msg combined
  local -a parts
  n="$(jq 'length' <<<"$ATOMIC_SWAPS")"
  if [[ "$n" == "0" ]]; then
    echo "  skip atomic swap (both selected pools already on peg)"
    SWAP_TX=""
    return 0
  fi
  echo "  atomic tx: $n pool-only swap(s) (no messages between pairs)"
  jq -c '.[] | {label,offer,amount,projected,rel}' <<<"$ATOMIC_SWAPS"
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "  [DRY_RUN] would broadcast $n MsgExecuteContract in one admin tx"
    SWAP_TX="DRY_RUN_TX"
    return 0
  fi
  workdir="$(mktemp -d "${TMPDIR:-/tmp}/oracle-rb-swaps.XXXXXX")"
  i=0
  parts=()
  while IFS= read -r row; do
    i=$((i + 1))
    contract="$(jq -r '.cw20' <<<"$row")"
    msg="$(cw20_swap_msg "$(jq -r '.pair' <<<"$row")" "$(jq -r '.amount' <<<"$row")")"
    generate_admin_unsigned "$contract" "$msg" "$workdir/u$i.json"
    parts+=("$workdir/u$i.json")
  done < <(jq -c '.[]' <<<"$ATOMIC_SWAPS")
  if [[ "$n" == "1" ]]; then
    combined="${parts[0]}"
  else
    combined="$workdir/combined.json"
    merge_unsigned_txs "$combined" "${parts[@]}" >/dev/null
    n="$(jq '.body.messages | length' "$combined")"
    [[ "$n" == "$(jq 'length' <<<"$ATOMIC_SWAPS")" ]] \
      || die "merged message count $n != planned swaps"
  fi
  SWAP_TX="$(broadcast_admin_unsigned "atomic oracle swaps ($n msgs)" "$combined")"
  rm -rf "$workdir"
}

burn_admin_leftovers() {
  local fail=0
  [[ "$ADMIN_ADDR" != "$TREASURY" ]] || die "refusing to burn on CMM treasury"
  refresh_balances
  echo "[burn] leftover UST1 / cUSTC / cLUNC on $ADMIN_ADDR (not LP, not USTR, not native)"
  echo "  before UST1=$BAL_UST1 cUSTC=$BAL_CUSTC cLUNC=$BAL_CLUNC"
  burn_if_any UST1 "$UST1" "$BAL_UST1"
  burn_if_any cUSTC "$CUSTC" "$BAL_CUSTC"
  burn_if_any cLUNC "$CLUNC" "$BAL_CLUNC"
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    return 0
  fi
  refresh_balances
  if [[ "$BAL_UST1" == "0" && "$BAL_CUSTC" == "0" && "$BAL_CLUNC" == "0" ]]; then
    echo "  PASS admin UST1/cUSTC/cLUNC = 0"
  else
    echo "  FAIL leftover remains UST1=$BAL_UST1 cUSTC=$BAL_CUSTC cLUNC=$BAL_CLUNC" >&2
    fail=1
  fi
  return "$fail"
}

echo "=============================================="
echo "Oracle rebalance (mint + swap + burn leftover)"
echo "=============================================="
echo "Pairs:      $PAIRS  (UST1/cUSTC=$DO_UST1 cLUNC/cUSTC=$DO_CLUNC)"
echo "Peg:        1 UST1 = \$1; cLUNC/cUSTC = LUNC_USD / USTC_USD"
echo "NO LP:      leftover wrap is burned, not provided"
echo "Atomic:     both pool swaps in one admin tx (mint is a prior tx; no AMM move)"
echo "Swap gas:   $SWAP_GAS"
echo "Factory:    $FACTORY"
echo "Treasury:   $TREASURY (never the burn target)"
echo "Admin:      $ADMIN_KEY ($ADMIN_ADDR)"
echo "2-of-3:     $MSIG_KEY ($MSIG_ADDR)"
echo "Minter:     $MINTER_KEY ($MINTER_ADDR)  mode=$MINT_MODE"
echo "Tolerance:  $TOLERANCE"
echo "BURN_ONLY:  ${ORACLE_RB_BURN_ONLY:-0}"
echo "DRY_RUN:    ${DRY_RUN:-0}"
echo ""

command -v jq >/dev/null || die "jq required"
command -v python3 >/dev/null || die "python3 required"
command -v curl >/dev/null || die "curl required"
[[ -f "$UST1_MATH" && -f "$CLUNC_MATH" ]] || die "missing math helpers"
python3 "$UST1_MATH" --self-test
python3 "$CLUNC_MATH" --self-test
if [[ "${DRY_RUN:-0}" != "1" ]]; then
  command -v terrad >/dev/null || die "terrad required on PATH"
fi

[[ "$ADMIN_ADDR" != "$TREASURY" ]] || die "ops wallet must not be the CMM treasury"

echo "[preflight] tokens"
SYM_UST1="$(token_symbol "$UST1")"
SYM_CUSTC="$(token_symbol "$CUSTC")"
SYM_CLUNC="$(token_symbol "$CLUNC")"
DEC_UST1="$(token_decimals "$UST1")"
DEC_CUSTC="$(token_decimals "$CUSTC")"
DEC_CLUNC="$(token_decimals "$CLUNC")"
echo "  UST1  $SYM_UST1 decimals=$DEC_UST1"
echo "  cUSTC $SYM_CUSTC decimals=$DEC_CUSTC"
echo "  cLUNC $SYM_CLUNC decimals=$DEC_CLUNC"
[[ "$(printf '%s' "$SYM_UST1" | tr '[:upper:]' '[:lower:]')" == "ust1" ]] || die "UST1 symbol $SYM_UST1"
[[ "$(printf '%s' "$SYM_CUSTC" | tr '[:upper:]' '[:lower:]')" == "custc" ]] || die "cUSTC symbol $SYM_CUSTC"
[[ "$(printf '%s' "$SYM_CLUNC" | tr '[:upper:]' '[:lower:]')" == "clunc" ]] || die "cLUNC symbol $SYM_CLUNC"

if [[ "$DO_UST1" == "1" ]]; then
  is_minter "$UST1" "$MSIG_ADDR" || die "UST1 minters do not include DEX 2-of-3 $MSIG_ADDR"
  is_minter "$CUSTC" "$MSIG_ADDR" || die "cUSTC minters do not include DEX 2-of-3 $MSIG_ADDR"
fi
if [[ "$DO_CLUNC" == "1" && "$MINT_MODE" == "minter" ]]; then
  is_minter "$CLUNC" "$MINTER_ADDR" || die "cLUNC minter is not $MINTER_ADDR"
  if [[ "$DO_UST1" != "1" ]]; then
    is_minter "$CUSTC" "$MINTER_ADDR" || die "cUSTC minter is not $MINTER_ADDR"
  fi
  echo "  WARN unbacked minter mint increases wrap CW20 supply without treasury native (W1)."
fi

PAIR_U_INFO="$(lcd_smart "$PAIR_UST1" '{"pair":{}}')"
LP_UST1="$(jq -r '.liquidity_token // empty' <<<"$PAIR_U_INFO")"
[[ -n "$LP_UST1" ]] || die "missing UST1/cUSTC LP token"
PAIR_C_INFO="$(lcd_smart "$PAIR_CLUNC" '{"pair":{}}')"
LP_CLUNC="$(jq -r '.liquidity_token // empty' <<<"$PAIR_C_INFO")"
[[ "$LP_CLUNC" == "$LP_CLUNC_PIN" ]] || die "cLUNC/cUSTC LP $LP_CLUNC != pin $LP_CLUNC_PIN"

FEE_UST1="$(lcd_smart "$PAIR_UST1" '{"get_fee_config":{}}' | jq -r '.fee_config.fee_bps // .fee_bps')"
FEE_CLUNC="$(lcd_smart "$PAIR_CLUNC" '{"get_fee_config":{}}' | jq -r '.fee_config.fee_bps // .fee_bps')"
MAPPER_CFG="$(lcd_smart "$WRAP_MAPPER" '{"config":{}}')"
[[ "$(jq -r '.paused' <<<"$MAPPER_CFG")" != "true" ]] || die "wrap-mapper is paused"
FEE_WRAP_BPS="$(jq -r '.fee_wrap_bps // .fee_bps' <<<"$MAPPER_CFG")"
[[ -n "$FEE_WRAP_BPS" && "$FEE_WRAP_BPS" != "null" ]] || die "wrap fee missing"

refresh_ust1_pool
refresh_clunc_pool
refresh_balances
echo "  UST1/cUSTC  $PAIR_UST1  reserves $R_UST1 / $R_UST1_Q  fee_bps=$FEE_UST1  px=$PX_UST1"
echo "  cLUNC/cUSTC $PAIR_CLUNC reserves $R_CLUNC / $R_CLUNC_Q  fee_bps=$FEE_CLUNC  px=$PX_CLUNC"
echo "  admin UST1=$BAL_UST1 cUSTC=$BAL_CUSTC cLUNC=$BAL_CLUNC"

echo ""
echo "[oracle] indexer + live CEX (1 UST1 = \$1)"
ORACLE_JSON="$(fetch_oracles_ok)" || {
  echo "$ORACLE_JSON" >&2
  die "oracle fetch / cross-check failed (set ORACLE_RB_SKIP_CROSS_CHECK=1 to bypass live CEX)"
}
echo "$ORACLE_JSON" | jq '{
  ok, lunc_usd, ustc_usd, target_custc_per_clunc,
  live_median, errors, warnings
}'
LUNC_USD="$(jq -r '.lunc_usd' <<<"$ORACLE_JSON")"
USTC_USD="$(jq -r '.ustc_usd' <<<"$ORACLE_JSON")"
TARGET_CLUNC="$(jq -r '.target_custc_per_clunc' <<<"$ORACLE_JSON")"
TARGET_UST1="$(python3 -c "from decimal import Decimal; print(Decimal(1) / Decimal('$USTC_USD'))")"
echo "  target UST1/cUSTC cUSTC-per-UST1 = $TARGET_UST1  (1 / $USTC_USD)"
echo "  target cLUNC/cUSTC cUSTC-per-cLUNC = $TARGET_CLUNC  ($LUNC_USD / $USTC_USD)"

if [[ "$DO_CLUNC" == "1" ]]; then
  if python3 -c "from decimal import Decimal; import sys; sys.exit(0 if Decimal('$PX_CLUNC') > Decimal('$TARGET_CLUNC') else 1)"; then
    echo "  this run will sell the cLUNC premium (live $PX_CLUNC > oracle $TARGET_CLUNC)"
  fi
fi

if [[ "${DRY_RUN:-0}" != "1" ]]; then
  prompt_keyring_pass
  TERRAD_HOST_KEY="$ADMIN_KEY"
  TERRAD_HOST_EXPECTED_ADDR="$ADMIN_ADDR"
  GOT_ADMIN="$(terrad_host_key_address)" || exit 1
  [[ "$GOT_ADMIN" == "$ADMIN_ADDR" ]] || die "admin key $ADMIN_KEY is $GOT_ADMIN (want $ADMIN_ADDR)"
  if [[ "$DO_CLUNC" == "1" && "$MINT_MODE" == "minter" && "$MINTER_ADDR" != "$ADMIN_ADDR" ]]; then
    TERRAD_HOST_KEY="$MINTER_KEY"
    TERRAD_HOST_EXPECTED_ADDR="$MINTER_ADDR"
    GOT_MINTER="$(terrad_host_key_address)" || exit 1
    [[ "$GOT_MINTER" == "$MINTER_ADDR" ]] || die "minter key $MINTER_KEY is $GOT_MINTER (want $MINTER_ADDR)"
  fi
else
  echo "  [DRY_RUN] skipping key unlock; assuming admin $ADMIN_ADDR"
fi

if [[ "${ORACLE_RB_BURN_ONLY:-0}" == "1" ]]; then
  if [[ "${DRY_RUN:-0}" != "1" && "${ORACLE_RB_YES:-0}" != "1" ]]; then
    if [[ ! -t 0 ]]; then
      die "refusing live burn without TTY; set ORACLE_RB_YES=1"
    fi
    read -r -p "Broadcast holder burn of leftover UST1/cUSTC/cLUNC on $ADMIN_ADDR ($TERRAD_HOST_CHAIN_ID)? [y/N] " ans
    [[ "$ans" == "y" || "$ans" == "Y" ]] || die "aborted"
  fi
  echo ""
  if ! burn_admin_leftovers; then
    die "leftover wrap burn failed"
  fi
  echo "OK — admin leftover UST1/cUSTC/cLUNC burned."
  exit 0
fi

echo ""
echo "[plan] swap-only atomic (usd_lp=0)"
plan_atomic_swaps
if [[ "$DO_UST1" == "1" ]]; then
  PLAN_U="$(plan_ust1)"
  echo "  UST1/cUSTC"
  jq '{
    target_custc_per_ust1, current_custc_per_ust1, current_custc_rel_error,
    already_on_peg, swap, mint, lp_custc
  }' <<<"$PLAN_U"
  [[ "$(jq -r '.lp_custc.ust1' <<<"$PLAN_U")" == "0" ]] \
    || die "planner wanted UST1/cUSTC LP despite swap-only"
fi
if [[ "$DO_CLUNC" == "1" ]]; then
  PLAN_C="$(plan_clunc)"
  echo "  cLUNC/cUSTC"
  jq '{
    target_custc_per_clunc, current_custc_per_clunc, current_rel_error,
    already_on_peg, add_usd, swap, mint, lp
  }' <<<"$PLAN_C"
  [[ "$(jq -r '.lp.clunc' <<<"$PLAN_C")" == "0" && "$(jq -r '.add_usd' <<<"$PLAN_C")" == "0" ]] \
    || die "planner wanted cLUNC/cUSTC LP despite force_add_usd=0"
fi
echo "  atomic swaps: $(jq -c '[.[] | .label]' <<<"$ATOMIC_SWAPS")"
echo "  mint need UST1=$NEED_UST1 cUSTC=$NEED_CUSTC cLUNC=$NEED_CLUNC"

if [[ "$DO_CLUNC" == "1" && "$MINT_MODE" == "wrap" ]]; then
  NEED_ULUNA="$(jq -r '.wrap_native.uluna // "0"' <<<"${PLAN_C:-{\"wrap_native\":{\"uluna\":\"0\"}}}")"
  NEED_UUSD="0"
  if [[ "$DO_UST1" != "1" ]]; then
    NEED_UUSD="$(jq -r '.wrap_native.uusd // "0"' <<<"$PLAN_C")"
  fi
  HAVE_ULUNA="$(lcd_bank "$ADMIN_ADDR" uluna)"
  HAVE_UUSD="$(lcd_bank "$ADMIN_ADDR" uusd)"
  echo "  wrap native need uluna=$NEED_ULUNA (have $HAVE_ULUNA, gas reserve $GAS_RESERVE_ULUNA)"
  echo "  wrap native need uusd=$NEED_UUSD (have $HAVE_UUSD)"
  python3 -c "import sys; sys.exit(0 if int('$HAVE_ULUNA') >= int('$NEED_ULUNA') + int('$GAS_RESERVE_ULUNA') else 1)" \
    || die "admin uluna short for wrap + gas"
  python3 -c "import sys; sys.exit(0 if int('$HAVE_UUSD') >= int('$NEED_UUSD') else 1)" \
    || die "admin uusd short for wrap"
fi

if [[ "${DRY_RUN:-0}" == "1" ]]; then
  echo ""
  echo "DRY_RUN complete (no txs). Live run mints (if needed), re-queries, then"
  echo "broadcasts both pool swaps in one admin tx."
  echo "Re-run without DRY_RUN=1 to broadcast."
  exit 0
fi

if [[ "${ORACLE_RB_YES:-0}" != "1" ]]; then
  if [[ ! -t 0 ]]; then
    die "refusing live broadcast without TTY; set ORACLE_RB_YES=1"
  fi
  read -r -p "Broadcast mint (if needed) then ONE atomic pool-swap tx (NO LP) on $TERRAD_HOST_CHAIN_ID? [y/N] " ans
  [[ "$ans" == "y" || "$ans" == "Y" ]] || die "aborted"
fi

FAIL=0
echo ""
echo "[mint] inventory for atomic swap (does not move AMM prices)"
fund_atomic_inventory

echo ""
echo "[atomic swap] re-query then one admin tx"
ORACLE_JSON="$(fetch_oracles_ok)" || die "oracle refresh failed immediately before atomic swap"
LUNC_USD="$(jq -r '.lunc_usd' <<<"$ORACLE_JSON")"
USTC_USD="$(jq -r '.ustc_usd' <<<"$ORACLE_JSON")"
TARGET_CLUNC="$(jq -r '.target_custc_per_clunc' <<<"$ORACLE_JSON")"
TARGET_UST1="$(python3 -c "from decimal import Decimal; print(Decimal(1) / Decimal('$USTC_USD'))")"
echo "  oracle LUNC/USD=$LUNC_USD USTC/USD=$USTC_USD"
echo "  target UST1/cUSTC=$TARGET_UST1  cLUNC/cUSTC=$TARGET_CLUNC"
plan_atomic_swaps
refresh_balances
if python3 -c "import sys; sys.exit(0 if int('$BAL_UST1') < int('$NEED_UST1') or int('$BAL_CUSTC') < int('$NEED_CUSTC') or int('$BAL_CLUNC') < int('$NEED_CLUNC') else 1)"; then
  echo "  inventory short after mint vs re-plan; topping up once"
  fund_atomic_inventory
  plan_atomic_swaps
  refresh_balances
  python3 -c "import sys; sys.exit(0 if int('$BAL_UST1') >= int('$NEED_UST1') and int('$BAL_CUSTC') >= int('$NEED_CUSTC') and int('$BAL_CLUNC') >= int('$NEED_CLUNC') else 1)" \
    || die "admin inventory still short for atomic swap (UST1 $BAL_UST1 need $NEED_UST1; cUSTC $BAL_CUSTC need $NEED_CUSTC; cLUNC $BAL_CLUNC need $NEED_CLUNC)"
fi
execute_atomic_swaps
echo "  swap_tx=${SWAP_TX:-skipped}"

echo ""
echo "[verify] oracle peg (no LP)"
if [[ "$DO_UST1" == "1" ]]; then
  refresh_ust1_pool
  if rel_err_ok "$PX_UST1" "$TARGET_UST1"; then
    echo "  PASS UST1/cUSTC $PX_UST1 within $TOLERANCE of $TARGET_UST1"
  else
    echo "  FAIL UST1/cUSTC $PX_UST1 vs $TARGET_UST1" >&2
    FAIL=1
  fi
fi
if [[ "$DO_CLUNC" == "1" ]]; then
  refresh_clunc_pool
  if rel_err_ok "$PX_CLUNC" "$TARGET_CLUNC"; then
    echo "  PASS cLUNC/cUSTC $PX_CLUNC within $TOLERANCE of $TARGET_CLUNC"
  else
    echo "  FAIL cLUNC/cUSTC $PX_CLUNC vs $TARGET_CLUNC" >&2
    FAIL=1
  fi
fi

ADM_LP_U="$(cw20_balance "$LP_UST1" "$ADMIN_ADDR")"
ADM_LP_C="$(cw20_balance "$LP_CLUNC" "$ADMIN_ADDR")"
if [[ "$ADM_LP_U" == "0" && "$ADM_LP_C" == "0" ]]; then
  echo "  PASS admin holds no LP (none was provided)"
else
  echo "  FAIL admin holds LP UST1/cUSTC=$ADM_LP_U cLUNC/cUSTC=$ADM_LP_C (not burned)" >&2
  FAIL=1
fi

if [[ "$FAIL" -ne 0 ]]; then
  die "post-checks failed (leftover wrap not burned; re-run)"
fi

if [[ "${ORACLE_RB_SKIP_BURN:-0}" == "1" ]]; then
  echo ""
  echo "SKIP leftover burn (ORACLE_RB_SKIP_BURN=1); inventory remains on $ADMIN_ADDR"
  echo "OK — pools on oracle peg; leftover wrap not burned."
  exit 0
fi

echo ""
if ! burn_admin_leftovers; then
  die "post-checks passed but leftover wrap burn failed"
fi
echo "OK — UST1/cUSTC and/or cLUNC/cUSTC on oracle peg; leftover UST1/cUSTC/cLUNC burned."
