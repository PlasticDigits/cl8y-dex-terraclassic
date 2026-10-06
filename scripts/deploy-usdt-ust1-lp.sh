#!/usr/bin/env bash
# Deploy USDT-Bridge-Debt, mint 10,000 USDT + 10,000 UST1, create the v2 pair,
# and move the LP token to the CMM treasury.
#
# Who signs what (keys already on this machine):
#   cl8ydeploy     store + instantiate the two new contracts
#   multisig_2of3  USDT add_minter, approve code, register + cap market, Issue 10k
#                  (signers: multisig1 + multisig2)
#   cl8y2_admin    mint 10k UST1 to itself, create the pair, provide LP to itself,
#                  then CW20-transfer that LP to the CMM treasury
#
# The USDT and UST1 inventory and the LP mint never land on the treasury first.
# USDT-Bridge-Debt is the exception: Issue mints it straight to the CMM, and the
# token cannot be transferred.
#
# Usage (from the repo root):
#   ./scripts/deploy-usdt-ust1-lp.sh                  # dry run (queries only)
#   BUILD_WASM=1 ./scripts/deploy-usdt-ust1-lp.sh     # workspace optimizer, still dry
#   USDT_UST1_LP_YES=1 ./scripts/deploy-usdt-ust1-lp.sh
#
# Unlock once (optional; otherwise the script prompts on the TTY):
#   read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
#
# Never commit TERRAD_HOST_KEYRING_PASS.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CONTRACTS_DIR="$REPO_ROOT/smartcontracts"
ARTIFACTS="$CONTRACTS_DIR/artifacts"
STATE_FILE="$REPO_ROOT/deployments/usdt-ust1-lp/state.env"

CHAIN_ID="${CHAIN_ID:-columbus-5}"
RPC="${RPC:-https://terra-classic-rpc.publicnode.com:443}"
LCD="${LCD:-https://terra-classic-lcd.publicnode.com}"
KEY_HOME="${KEY_HOME:-$HOME/.terra}"
KEYRING_BACKEND="${KEYRING_BACKEND:-}"
GAS_PRICES="${GAS_PRICES:-28.325uluna}"
GAS_ADJUSTMENT="${GAS_ADJUSTMENT:-1.4}"

DEPLOYER_KEY="${DEPLOYER_KEY:-cl8ydeploy}"
DEPLOYER_ADDR="${DEPLOYER_ADDR:-terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv}"
ADMIN_KEY="${ADMIN_KEY:-cl8y2_admin}"
ADMIN_ADDR="${ADMIN_ADDR:-terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l}"
MSIG_KEY="${MSIG_KEY:-multisig_2of3}"
MSIG_ADDR="${MSIG_ADDR:-terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7}"
SIGNER1="${SIGNER1:-multisig1}"
SIGNER2="${SIGNER2:-multisig2}"

FACTORY="${FACTORY:-terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea}"
USDT="${USDT:-terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4}"
UST1="${UST1:-terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72}"
TREASURY="${TREASURY:-terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2}"

# 10_000 whole tokens. USDT is 18 decimals; UST1 is 6.
USDT_RAW="${USDT_RAW:-10000000000000000000000}"
UST1_RAW="${UST1_RAW:-10000000000}"
# Review input, not a value learned from the implementation being approved.
REVIEWED_USDT_CODE_ID="${REVIEWED_USDT_CODE_ID:-10184}"

LIVE=0
if [[ "${USDT_UST1_LP_YES:-0}" == "1" ]]; then
  LIVE=1
fi

die() { echo "ERROR: $*" >&2; exit 1; }

lcd_b64() { printf '%s' "$1" | base64 -w0; }

lcd_get() {
  curl -fsS --connect-timeout 10 --max-time 25 "$1"
}

lcd_smart_raw() {
  local contract="$1" msg="$2" url code body
  url="${LCD%/}/cosmwasm/wasm/v1/contract/${contract}/smart/$(lcd_b64 "$msg")"
  body="$(curl -sS --connect-timeout 10 --max-time 25 -w $'\n%{http_code}' "$url")" || true
  code="${body##*$'\n'}"
  body="${body%$'\n'*}"
  printf '%s' "$body"
  [[ "$code" == "200" ]]
}

lcd_smart() {
  local raw="$1"
  if echo "$raw" | jq -e '.data' >/dev/null 2>&1; then
    if [[ "$(jq -r '.data | type' <<<"$raw")" == "string" ]]; then
      jq -r '.data | @base64d | fromjson' <<<"$raw"
    else
      jq '.data' <<<"$raw"
    fi
    return 0
  fi
  echo "$raw" >&2
  return 1
}

cw20_balance() {
  local raw
  raw="$(lcd_smart_raw "$1" "$(jq -nc --arg a "$2" '{balance:{address:$a}}')")" \
    || die "balance query failed for $1"
  lcd_smart "$raw" | jq -r '.balance // "0"'
}

token_supply() {
  local raw
  raw="$(lcd_smart_raw "$1" '{"token_info":{}}')" || die "token_info failed for $1"
  lcd_smart "$raw" | jq -r '.total_supply // "0"'
}

py_ge() {
  python3 -c "import sys; sys.exit(0 if int(sys.argv[1]) >= int(sys.argv[2]) else 1)" "$1" "$2"
}

resolve_keyring() {
  if [[ -n "$KEYRING_BACKEND" ]]; then
    return 0
  fi
  if [[ -f "$KEY_HOME/keyring-file/${ADMIN_KEY}.info" ]]; then
    KEYRING_BACKEND=file
  elif [[ -f "$KEY_HOME/keyring-test/${ADMIN_KEY}.info" ]]; then
    KEYRING_BACKEND=test
  else
    KEYRING_BACKEND=os
  fi
}

prompt_pass() {
  resolve_keyring
  [[ "$KEYRING_BACKEND" == "file" ]] || return 0
  [[ "$LIVE" == "1" ]] || return 0
  if [[ -n "${TERRAD_HOST_KEYRING_PASS:-}" ]]; then
    return 0
  fi
  [[ -r /dev/tty && -w /dev/tty ]] || die "set TERRAD_HOST_KEYRING_PASS (no TTY)"
  printf 'terrad keyring passphrase: ' >/dev/tty
  read -rs TERRAD_HOST_KEYRING_PASS </dev/tty
  printf '\n' >/dev/tty
  export TERRAD_HOST_KEYRING_PASS
  [[ -n "$TERRAD_HOST_KEYRING_PASS" ]] || die "empty passphrase"
}

# Passphrase is piped, never printed. pipefail is off so a closed stdin is not a failure.
terrad_exec() {
  if [[ -z "${TERRAD_HOST_KEYRING_PASS:-}" ]]; then
    terrad "$@"
    return $?
  fi
  local st pipefail_was=0
  [[ -o pipefail ]] && pipefail_was=1
  set +o pipefail
  {
    local _i
    for _i in $(seq 1 64); do
      printf '%s\n' "$TERRAD_HOST_KEYRING_PASS" || break
    done
  } | terrad "$@"
  st=${PIPESTATUS[1]}
  [[ "$pipefail_was" -eq 1 ]] && set -o pipefail
  return "$st"
}

# Publicnode returns 503 "upstream temporarily unavailable" on wasm-store
# simulation. Retry that class of error. Stdout is emitted only after success
# so a failed attempt cannot pollute a captured tx or --generate-only file.
# Exit 75 means the retries were all transient; callers may fall back.
terrad_exec_retry() {
  local attempts="${TERRAD_HOST_TX_RETRIES:-8}"
  local delay="${TERRAD_HOST_TX_RETRY_DELAY_SEC:-4}"
  local i out st errf
  errf="$(mktemp)"
  for i in $(seq 1 "$attempts"); do
    set +e
    out="$(terrad_exec "$@" 2>"$errf")"
    st=$?
    set -e
    if [[ "$st" -eq 0 ]]; then
      printf '%s' "$out"
      rm -f "$errf"
      return 0
    fi
    if grep -qiE 'connection reset|EOF|i/o timeout|broken pipe|TLS handshake|temporarily unavailable|503|502|429|account sequence mismatch|incorrect account sequence' "$errf"; then
      echo "[deploy] transient RPC (attempt $i/$attempts), retry in ${delay}s" >&2
      head -c 400 "$errf" >&2
      echo >&2
      sleep "$delay"
      delay=$((delay + 2))
      continue
    fi
    cat "$errf" >&2
    rm -f "$errf"
    return "$st"
  done
  echo "[deploy] exhausted $attempts RPC retries" >&2
  cat "$errf" >&2 || true
  rm -f "$errf"
  return 75
}

common_flags() {
  resolve_keyring
  echo --chain-id "$CHAIN_ID" --node "$RPC" --keyring-backend "$KEYRING_BACKEND" --home "$KEY_HOME"
}

fee_flags() {
  if [[ -n "${GAS_LIMIT:-}" ]]; then
    echo --gas "$GAS_LIMIT" --gas-prices "$GAS_PRICES"
  else
    echo --gas auto --gas-adjustment "$GAS_ADJUSTMENT" --gas-prices "$GAS_PRICES"
  fi
}

wait_tx() {
  local txhash="$1" attempt=1 tx_json
  while [[ "$attempt" -le 40 ]]; do
    if tx_json="$(terrad query tx "$txhash" --node "$RPC" --output json 2>/dev/null)"; then
      if echo "$tx_json" | jq -e '.code == 0 or .code == "0"' >/dev/null; then
        printf '%s' "$tx_json"
        return 0
      fi
      echo "$tx_json" | jq -r '.raw_log // .log // empty' >&2
      die "tx $txhash failed"
    fi
    sleep 2
    attempt=$((attempt + 1))
  done
  die "tx $txhash not indexed"
}

event_attr() {
  local tx_json="$1" event_type="$2" attr_key="$3"
  echo "$tx_json" | jq -r --arg t "$event_type" --arg k "$attr_key" '
    def all_events: (.events // []) + ([.logs[]?.events // []] | add // []);
    [all_events[] | select(.type == $t) | .attributes[] | select(.key == $k) | .value][0] // empty'
}

broadcast_key() {
  local key="$1" label="$2"
  shift 2
  echo "  → $key: $label" >&2
  if [[ "$LIVE" != "1" ]]; then
    echo "    [dry] terrad tx $*" >&2
    echo "DRY_RUN_TX"
    return 0
  fi
  local out txhash result st
  set +e
  # shellcheck disable=SC2046
  out="$(terrad_exec_retry tx "$@" --from "$key" $(common_flags) $(fee_flags) \
    --broadcast-mode sync -y --output json)"
  st=$?
  set -e
  if [[ "$st" -ne 0 ]]; then
    return "$st"
  fi
  txhash="$(printf '%s' "$out" | jq -r '.txhash // empty')"
  [[ -n "$txhash" ]] || { printf '%s\n' "$out" >&2; die "no txhash: $label"; }
  local code
  code="$(printf '%s' "$out" | jq -r '.code // 0')"
  [[ "$code" == "0" ]] || { printf '%s\n' "$out" >&2; die "broadcast $label code $code"; }
  echo "    tx $txhash" >&2
  result="$(wait_tx "$txhash")"
  printf '%s' "$result"
}

msig_exec() {
  local contract="$1" msg="$2" label="$3"
  echo "  → ${MSIG_KEY}: $label" >&2
  if [[ "$LIVE" != "1" ]]; then
    echo "    [dry] $contract $msg" >&2
    echo "DRY_RUN_TX"
    return 0
  fi
  local work unsigned sig1 sig2 signed out txhash code
  work="$(mktemp -d)"
  unsigned="$work/unsigned.json"
  sig1="$work/sig1.json"
  sig2="$work/sig2.json"
  signed="$work/signed.json"
  # shellcheck disable=SC2046
  terrad_exec_retry tx wasm execute "$contract" "$msg" --from "$MSIG_KEY" --generate-only \
    $(common_flags) $(fee_flags) --output json >"$unsigned" \
    || die "unsigned tx failed ($label)"
  jq -e '.body.messages | length > 0' "$unsigned" >/dev/null || die "unsigned tx empty ($label)"
  # shellcheck disable=SC2046
  terrad_exec_retry tx sign "$unsigned" --from "$SIGNER1" --multisig "$MSIG_KEY" \
    --sign-mode amino-json $(common_flags) --output json >"$sig1" \
    || die "sign $SIGNER1 failed ($label)"
  # shellcheck disable=SC2046
  terrad_exec_retry tx sign "$unsigned" --from "$SIGNER2" --multisig "$MSIG_KEY" \
    --sign-mode amino-json $(common_flags) --output json >"$sig2" \
    || die "sign $SIGNER2 failed ($label)"
  # shellcheck disable=SC2046
  terrad_exec_retry tx multisign "$unsigned" "$MSIG_KEY" "$sig1" "$sig2" \
    $(common_flags) --output json >"$signed" \
    || die "multisign failed ($label)"
  # shellcheck disable=SC2046
  out="$(terrad_exec_retry tx broadcast "$signed" $(common_flags) --broadcast-mode sync -y --output json)" \
    || die "multisig broadcast failed ($label)"
  txhash="$(printf '%s' "$out" | jq -r '.txhash // empty')"
  code="$(printf '%s' "$out" | jq -r '.code // 0')"
  [[ -n "$txhash" && "$code" == "0" ]] || { printf '%s\n' "$out" >&2; die "multisig broadcast failed ($label)"; }
  echo "    tx $txhash" >&2
  wait_tx "$txhash" >/dev/null
  rm -rf "$work"
  printf '%s' "$txhash"
}

save_state() {
  mkdir -p "$(dirname "$STATE_FILE")"
  cat >"$STATE_FILE" <<EOF
ISSUER=${ISSUER:-}
DEBT=${DEBT:-}
DEBT_CODE_ID=${DEBT_CODE_ID:-}
ISSUER_CODE_ID=${ISSUER_CODE_ID:-}
PAIR=${PAIR:-}
LP_TOKEN=${LP_TOKEN:-}
EOF
}

load_state() {
  if [[ -f "$STATE_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$STATE_FILE"
  fi
  ISSUER="${ISSUER:-}"
  DEBT="${DEBT:-}"
  DEBT_CODE_ID="${DEBT_CODE_ID:-}"
  ISSUER_CODE_ID="${ISSUER_CODE_ID:-}"
  PAIR="${PAIR:-}"
  LP_TOKEN="${LP_TOKEN:-}"
}

build_wasm() {
  if [[ "${BUILD_WASM:-0}" != "1" ]]; then
    return 0
  fi
  echo "[build] smartcontracts/scripts/optimize.sh"
  (cd "$CONTRACTS_DIR" && bash scripts/optimize.sh)
}

require_wasm() {
  [[ -f "$ARTIFACTS/bridge_debt_token.wasm" && -f "$ARTIFACTS/bridge_debt.wasm" ]] \
    || die "missing wasm in $ARTIFACTS. Run: BUILD_WASM=1 ./scripts/deploy-usdt-ust1-lp.sh"
}

store_code() {
  local wasm="$1" label="$2" tx code_id st
  local store_gas="${STORE_GAS_LIMIT:-25000000}"
  echo "[store] $label" >&2
  set +e
  tx="$(broadcast_key "$DEPLOYER_KEY" "store $label" wasm store "$wasm")"
  st=$?
  set -e
  if [[ "$st" -eq 75 ]]; then
    echo "[store] simulation RPC stayed unavailable; broadcasting $label with --gas $store_gas" >&2
    tx="$(GAS_LIMIT="$store_gas" broadcast_key "$DEPLOYER_KEY" "store $label" wasm store "$wasm")" \
      || exit 1
  elif [[ "$st" -ne 0 ]]; then
    exit "$st"
  fi
  code_id="$(event_attr "$tx" store_code code_id)"
  [[ -n "$code_id" ]] || die "no code_id from store $label"
  echo "  code_id=$code_id" >&2
  printf '%s' "$code_id"
}

echo "=============================================="
echo "USDT/UST1 v2 LP  10,000 + 10,000  → CMM"
echo "=============================================="
echo "Live:     $LIVE   (set USDT_UST1_LP_YES=1 to broadcast)"
echo "Admin:    $ADMIN_KEY ($ADMIN_ADDR)   UST1 mint, pair, LP, transfer"
echo "Multisig: $MSIG_KEY ($MSIG_ADDR)   USDT Issue + debt registration"
echo "Deployer: $DEPLOYER_KEY ($DEPLOYER_ADDR)"
echo "Treasury: $TREASURY"
echo "USDT raw: $USDT_RAW"
echo "UST1 raw: $UST1_RAW"
echo ""

load_state
build_wasm

if [[ "$LIVE" == "1" && ( -z "$DEBT_CODE_ID" || -z "$ISSUER_CODE_ID" ) ]]; then
  require_wasm
fi

echo "[preflight] tokens, whitelist, pair"
USDT_INFO="$(lcd_smart "$(lcd_smart_raw "$USDT" '{"token_info":{}}')")"
UST1_INFO="$(lcd_smart "$(lcd_smart_raw "$UST1" '{"token_info":{}}')")"
[[ "$(jq -r '.decimals' <<<"$USDT_INFO")" == "18" ]] || die "USDT decimals changed"
[[ "$(jq -r '.decimals' <<<"$UST1_INFO")" == "6" ]] || die "UST1 decimals changed"
[[ "$(jq -r '.symbol' <<<"$USDT_INFO")" == "USDT" ]] || die "unexpected USDT symbol"
[[ "$(jq -r '.symbol' <<<"$UST1_INFO")" == "UST1" ]] || die "unexpected UST1 symbol"

USDT_CODE="$(lcd_get "${LCD%/}/cosmwasm/wasm/v1/contract/${USDT}" | jq -r '.contract_info.code_id')"
UST1_CODE="$(lcd_get "${LCD%/}/cosmwasm/wasm/v1/contract/${UST1}" | jq -r '.contract_info.code_id')"
[[ "$REVIEWED_USDT_CODE_ID" =~ ^[1-9][0-9]*$ && "$USDT_CODE" == "$REVIEWED_USDT_CODE_ID" ]] \
  || die "USDT code $USDT_CODE differs from reviewed code $REVIEWED_USDT_CODE_ID; review implementation before any transaction"
for id in "$USDT_CODE" "$UST1_CODE"; do
  ok="$(lcd_smart "$(lcd_smart_raw "$FACTORY" "$(jq -nc --argjson id "$id" '{is_code_id_whitelisted:{code_id:$id}}')")" | jq -r '.whitelisted')"
  [[ "$ok" == "true" ]] || die "code_id $id is not factory-whitelisted"
done
echo "  USDT code=$USDT_CODE  UST1 code=$UST1_CODE  both whitelisted"

FEE_ULUNA="$(lcd_smart "$(lcd_smart_raw "$FACTORY" '{"config":{}}')" | jq -r '.pair_creation_fee_uluna')"
echo "  pair_creation_fee_uluna=$FEE_ULUNA"

pair_lookup() {
  local a="$1" b="$2" raw
  raw="$(lcd_smart_raw "$FACTORY" "$(jq -nc --arg a "$a" --arg b "$b" \
    '{pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')")" || true
  if echo "$raw" | grep -qi 'pair not found'; then
    printf ''
    return 0
  fi
  lcd_smart "$raw" | jq -r '.pair.contract_addr // empty'
}

if [[ -z "$PAIR" ]]; then
  PAIR="$(pair_lookup "$UST1" "$USDT")"
  if [[ -z "$PAIR" ]]; then
    PAIR="$(pair_lookup "$USDT" "$UST1")"
  fi
fi
if [[ -n "$PAIR" ]]; then
  echo "  existing pair $PAIR"
else
  echo "  no USDT/UST1 pair yet"
fi

if [[ "$LIVE" == "1" ]]; then
  prompt_pass
fi

if [[ -z "$DEBT_CODE_ID" ]]; then
  if [[ "$LIVE" == "1" ]]; then
    DEBT_CODE_ID="$(store_code "$ARTIFACTS/bridge_debt_token.wasm" bridge-debt-token)" || exit 1
    save_state
  else
    echo "[dry] store artifacts/bridge_debt_token.wasm with $DEPLOYER_KEY"
  fi
fi
if [[ -z "$ISSUER_CODE_ID" ]]; then
  if [[ "$LIVE" == "1" ]]; then
    ISSUER_CODE_ID="$(store_code "$ARTIFACTS/bridge_debt.wasm" bridge-debt)" || exit 1
    save_state
  else
    echo "[dry] store artifacts/bridge_debt.wasm with $DEPLOYER_KEY"
  fi
fi

if [[ "$LIVE" == "1" && -z "$ISSUER" ]]; then
  # terrad instantiate does not take --amount. Store used wasm store; this one does not.
  msg="$(jq -nc --arg o "$MSIG_ADDR" '{owner:$o}')"
  tx="$(broadcast_key "$DEPLOYER_KEY" "instantiate bridge-debt" \
    wasm instantiate "$ISSUER_CODE_ID" "$msg" --label "bridge-debt" --admin "$MSIG_ADDR")" || exit 1
  ISSUER="$(event_attr "$tx" instantiate _contract_address)"
  [[ -n "$ISSUER" ]] || die "issuer instantiate returned no address"
  echo "  issuer=$ISSUER"
  save_state
elif [[ "$LIVE" != "1" && -z "$ISSUER" ]]; then
  echo "[dry] would instantiate bridge-debt code $ISSUER_CODE_ID owner $MSIG_ADDR"
fi

if [[ "$LIVE" == "1" && -z "$DEBT" ]]; then
  [[ -n "$ISSUER" ]] || die "issuer address missing"
  msg="$(jq -nc --arg m "$ISSUER" --arg d "$TREASURY" \
    '{name:"USDT-Bridge-Debt",symbol:"USDT-Bridge-Debt",decimals:18,minter:$m,debtor:$d,cap:null}')"
  tx="$(broadcast_key "$DEPLOYER_KEY" "instantiate USDT-Bridge-Debt" \
    wasm instantiate "$DEBT_CODE_ID" "$msg" --label "USDT-Bridge-Debt" --admin "$MSIG_ADDR")" || exit 1
  DEBT="$(event_attr "$tx" instantiate _contract_address)"
  [[ -n "$DEBT" ]] || die "debt instantiate returned no address"
  echo "  debt=$DEBT"
  save_state
elif [[ "$LIVE" != "1" && -z "$DEBT" ]]; then
  echo "[dry] would instantiate USDT-Bridge-Debt minter=<issuer> debtor=$TREASURY"
fi

if [[ "$LIVE" == "1" ]]; then
  [[ -n "$ISSUER" && -n "$DEBT" && -n "$DEBT_CODE_ID" ]] || die "deploy incomplete; re-run after fixing the failed step"
fi

echo "[multisig] USDT minter + debt market + Issue to $ADMIN_ADDR"
if [[ "$LIVE" == "1" ]]; then
  market_raw="$(lcd_smart_raw "$ISSUER" "$(jq -nc --arg u "$USDT" '{market:{underlying:$u}}')")" || true
  if echo "$market_raw" | grep -qi 'bridge_debt::state::Market.*not found\|Unknown market'; then
    cfg="$(lcd_smart "$(lcd_smart_raw "$ISSUER" '{"config":{}}')")"
    jq -e 'has("pending_owner")' <<<"$cfg" >/dev/null \
      || die "issuer lacks the market-pin protocol; review and migrate it before setup mutations"
    minters="$(lcd_smart "$(lcd_smart_raw "$USDT" '{"minters":{}}')")"
    primary="$(lcd_smart "$(lcd_smart_raw "$USDT" '{"minter":{}}')" | jq -r '.minter')"
    if [[ "$primary" != "$ISSUER" ]] && ! jq -e --arg m "$ISSUER" '.minters | index($m) != null' <<<"$minters" >/dev/null; then
      msig_exec "$USDT" "$(jq -nc --arg m "$ISSUER" '{add_minter:{minter:$m}}')" "add_minter issuer on USDT"
    fi
    if [[ "$(jq -r '.debt_code_id // empty' <<<"$cfg")" != "$DEBT_CODE_ID" ]]; then
      msig_exec "$ISSUER" "$(jq -nc --argjson id "$DEBT_CODE_ID" '{set_debt_code_id:{code_id:$id}}')" \
        "set_debt_code_id $DEBT_CODE_ID"
    fi
    # Approve reviewed code only for a new registration, never to repair drift.
    msig_exec "$ISSUER" "$(jq -nc --argjson id "$REVIEWED_USDT_CODE_ID" \
      '{approve_underlying_code:{code_id:$id,multi_minter:true}}')" \
      "approve reviewed USDT implementation $REVIEWED_USDT_CODE_ID (multi-minter)"
    msig_exec "$ISSUER" "$(jq -nc --arg u "$USDT" --arg d "$DEBT" \
      '{register_market:{underlying:$u,debt_token:$d}}')" "register USDT market"
  else
    lcd_smart "$market_raw" | jq -e '.debt_token' >/dev/null \
      || die "cannot determine existing market; refusing setup mutations"
    echo "  market already registered"
  fi

  market="$(lcd_smart "$(lcd_smart_raw "$ISSUER" "$(jq -nc --arg u "$USDT" '{market:{underlying:$u}}')")")"
  jq -e --arg d "$DEBT" --argjson code "$USDT_CODE" --argjson debt_code "$DEBT_CODE_ID" \
    '.debt_token == $d and .pins.underlying_code_id == $code and .pins.debt_code_id == $debt_code' \
    <<<"$market" >/dev/null || die "existing market identity/code pins differ; use reviewed upgrade procedure"
  [[ "$(jq -r '.paused' <<<"$market")" == "false" && "$(jq -r '.pending_upgrade == null' <<<"$market")" == "true" ]] \
    || die "market paused or pending upgrade; resolve separately before issuance"
  limits="$(jq -nc --arg a "$USDT_RAW" \
    '{max_per_issue:$a,max_outstanding:$a,epoch_seconds:86400,max_per_epoch:$a}')"
  if [[ "$(jq -cS '.limits' <<<"$market")" != "$(jq -cS . <<<"$limits")" ]]; then
    [[ "$(jq -r '.limits == null' <<<"$market")" == "true" ]] \
      || die "existing issuance limits differ; review separately instead of silently widening caps"
    msig_exec "$ISSUER" "$(jq -nc --arg u "$USDT" --argjson l "$limits" \
      '{configure_market:{underlying:$u,limits:$l,paused:false}}')" \
      "configure finite 10k USDT caps, fixed UTC 86400-second epochs"
  fi
else
  echo "  [dry] add_minter $ISSUER on USDT, set_debt_code_id, approve_underlying_code $USDT_CODE (multi_minter=true), register_market"
  echo "  [dry] configure_market max_per_issue=max_outstanding=max_per_epoch=$USDT_RAW, epoch_seconds=86400; issue"
fi

admin_usdt="$(cw20_balance "$USDT" "$ADMIN_ADDR")"
echo "  admin USDT now=$admin_usdt"
debt_now="0"
if [[ -n "${DEBT:-}" ]]; then
  debt_now="$(cw20_balance "$DEBT" "$TREASURY")"
fi
echo "  CMM USDT-Bridge-Debt now=$debt_now"
if py_ge "$debt_now" "$USDT_RAW"; then
  echo "  skip USDT issue (CMM already holds 10,000 debt)"
elif [[ "$LIVE" == "1" ]]; then
  [[ "$debt_now" == "0" ]] || die "partial existing debt requires a reviewed issuance plan; refusing to over-issue"
  msig_exec "$ISSUER" "$(jq -nc --arg u "$USDT" --arg r "$ADMIN_ADDR" --arg a "$USDT_RAW" \
    '{issue:{underlying:$u,recipient:$r,amount:$a}}')" \
    "issue $USDT_RAW USDT → $ADMIN_ADDR and debt → CMM"
else
  echo "  [dry] multisig Issue $USDT_RAW USDT to $ADMIN_ADDR and the same debt to $TREASURY"
fi

admin_ust1="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
short_ust1="$(python3 -c "print(max(0, int('$UST1_RAW') - int('$admin_ust1')))")"
echo "  admin UST1=$admin_ust1  shortfall=$short_ust1"
if py_ge "$short_ust1" 1; then
  broadcast_key "$ADMIN_KEY" "mint $short_ust1 UST1 → admin" \
    wasm execute "$UST1" "$(jq -nc --arg r "$ADMIN_ADDR" --arg a "$short_ust1" '{mint:{recipient:$r,amount:$a}}')" \
    >/dev/null
else
  echo "  skip UST1 mint (admin already holds 10,000)"
fi

if [[ "$LIVE" == "1" ]]; then
  admin_usdt="$(cw20_balance "$USDT" "$ADMIN_ADDR")"
  admin_ust1="$(cw20_balance "$UST1" "$ADMIN_ADDR")"
  py_ge "$admin_usdt" "$USDT_RAW" || die "admin USDT $admin_usdt < $USDT_RAW"
  py_ge "$admin_ust1" "$UST1_RAW" || die "admin UST1 $admin_ust1 < $UST1_RAW"
  debt_bal="$(cw20_balance "$DEBT" "$TREASURY")"
  echo "  CMM USDT-Bridge-Debt=$debt_bal"
  py_ge "$debt_bal" "$USDT_RAW" || die "CMM debt $debt_bal < $USDT_RAW (Issue did not mint the receipt)"
fi

echo "[pair] create on factory if missing, provide to admin, transfer LP to treasury"
if [[ -z "$PAIR" ]]; then
  if [[ "$LIVE" == "1" ]]; then
    create="$(jq -nc --arg a "$UST1" --arg b "$USDT" \
      '{create_pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
    broadcast_key "$ADMIN_KEY" "create_pair UST1/USDT" \
      wasm execute "$FACTORY" "$create" --amount "${FEE_ULUNA}uluna" >/dev/null
    sleep 3
    PAIR="$(pair_lookup "$UST1" "$USDT")"
    [[ -n "$PAIR" ]] || PAIR="$(pair_lookup "$USDT" "$UST1")"
    [[ -n "$PAIR" ]] || die "create_pair did not return a pair address"
    echo "  pair=$PAIR"
    save_state
  else
    echo "  [dry] create_pair fee ${FEE_ULUNA}uluna from $ADMIN_KEY"
  fi
fi

if [[ -n "$PAIR" && "$LIVE" == "1" ]]; then
  share="$(lcd_smart "$(lcd_smart_raw "$PAIR" '{"pool":{}}')" | jq -r '.total_share // "0"')"
  pair_info="$(lcd_smart "$(lcd_smart_raw "$PAIR" '{"pair":{}}')")"
  LP_TOKEN="$(jq -r '.liquidity_token' <<<"$pair_info")"
  t0="$(jq -r '.asset_infos[0].token.contract_addr' <<<"$pair_info")"
  t1="$(jq -r '.asset_infos[1].token.contract_addr' <<<"$pair_info")"
  save_state
  echo "  lp_token=$LP_TOKEN total_share=$share"
  if [[ "$share" != "0" ]]; then
    echo "  pool already has liquidity; will not provide again"
  else
    amt0="$UST1_RAW"
    amt1="$USDT_RAW"
    [[ "$t0" == "$USDT" ]] && amt0="$USDT_RAW"
    [[ "$t0" == "$UST1" ]] && amt0="$UST1_RAW"
    [[ "$t1" == "$USDT" ]] && amt1="$USDT_RAW"
    [[ "$t1" == "$UST1" ]] && amt1="$UST1_RAW"
    allow0="$(jq -nc --arg s "$PAIR" --arg a "$amt0" '{increase_allowance:{spender:$s,amount:$a,expires:{never:{}}}}')"
    allow1="$(jq -nc --arg s "$PAIR" --arg a "$amt1" '{increase_allowance:{spender:$s,amount:$a,expires:{never:{}}}}')"
    provide="$(jq -nc --arg a "$t0" --arg b "$t1" --arg aa "$amt0" --arg bb "$amt1" --arg recv "$ADMIN_ADDR" \
      '{provide_liquidity:{assets:[
        {info:{token:{contract_addr:$a}},amount:$aa},
        {info:{token:{contract_addr:$b}},amount:$bb}
      ],slippage_tolerance:null,receiver:$recv,deadline:null}}')"
    broadcast_key "$ADMIN_KEY" "allow $t0" wasm execute "$t0" "$allow0" >/dev/null
    broadcast_key "$ADMIN_KEY" "allow $t1" wasm execute "$t1" "$allow1" >/dev/null
    broadcast_key "$ADMIN_KEY" "provide_liquidity → admin" wasm execute "$PAIR" "$provide" >/dev/null
  fi

  lp_bal="$(cw20_balance "$LP_TOKEN" "$ADMIN_ADDR")"
  echo "  admin LP balance=$lp_bal"
  if py_ge "$lp_bal" 1; then
    broadcast_key "$ADMIN_KEY" "transfer LP → treasury" \
      wasm execute "$LP_TOKEN" "$(jq -nc --arg r "$TREASURY" --arg a "$lp_bal" '{transfer:{recipient:$r,amount:$a}}')" \
      >/dev/null
  fi
  treasury_lp="$(cw20_balance "$LP_TOKEN" "$TREASURY")"
  admin_left="$(cw20_balance "$LP_TOKEN" "$ADMIN_ADDR")"
  echo "  treasury LP=$treasury_lp  admin LP left=$admin_left"
  [[ "$admin_left" == "0" ]] || die "admin still holds LP ($admin_left)"
  py_ge "$treasury_lp" 1 || die "treasury LP balance is zero"

  whitelist="$(lcd_smart "$(lcd_smart_raw "$TREASURY" '{"cw20_whitelist":{}}')" | jq -r '.addresses[]?' || true)"
  for token in "$LP_TOKEN" "$DEBT"; do
    if [[ "$whitelist" == *"$token"* ]]; then
      echo "  treasury already whitelists $token"
    else
      msig_exec "$TREASURY" "$(jq -nc --arg c "$token" '{add_cw20:{contract_addr:$c}}')" "AddCw20 $token"
    fi
  done
elif [[ "$LIVE" != "1" ]]; then
  echo "  [dry] provide 10k/10k to $ADMIN_ADDR, then transfer the LP to $TREASURY"
fi

echo ""
if [[ "$LIVE" == "1" ]]; then
  echo "Done."
  echo "  issuer:   ${ISSUER:-}"
  echo "  debt:     ${DEBT:-}"
  echo "  pair:     ${PAIR:-}"
  echo "  lp:       ${LP_TOKEN:-}"
  echo "  treasury: $TREASURY"
else
  echo "Dry run only. Broadcast with: USDT_UST1_LP_YES=1 ./scripts/deploy-usdt-ust1-lp.sh"
fi
