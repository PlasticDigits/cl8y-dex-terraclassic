#!/usr/bin/env bash
# LocalTerra community-tax seed (GitLab #620).
#
# Source from scripts/deploy-dex-local.sh after factory + EMBER exist.
# Requires parent helpers: terrad_tx, terrad_query, wait_tx, get_code_id,
# get_contract_address, factory_create_pair.
#
# Whitelist **only** the LocalTerra store id. Never 11611/11619/launcher/AutoLP/8654.
# QA token is free of MintControl; funding tops up via Transfer from test1.
# CMM stand-in is test1 — not columbus-5 terra16j5u6….
# Local UST1 is a mintable invoice stand-in — not columbus-5 UST1 / window.

# Columbus-5 pins that must never be AddWhitelistedCodeId from this script.
_CTAX_FORBIDDEN_CODE_IDS="11611 11612 11613 11614 11619 11620 11621 11622 8654"

_ctax_is_forbidden_code() {
  local id="$1" x
  for x in $_CTAX_FORBIDDEN_CODE_IDS; do
    if [ "$id" = "$x" ]; then
      return 0
    fi
  done
  return 1
}

_ctax_find_wasm() {
  local name="$1"
  local cand
  for cand in \
    "${ARTIFACTS_DIR}/${name}.wasm" \
    "${REPO_ROOT}/smartcontracts/artifacts/${name}.wasm" \
    "${REPO_ROOT}/smartcontracts/target/wasm32-unknown-unknown/release/${name}.wasm"; do
    if [ -f "$cand" ]; then
      printf '%s' "$cand"
      return 0
    fi
  done
  return 1
}

_ctax_require_wasm() {
  local name="$1" pkg="$2"
  local path
  if path="$(_ctax_find_wasm "$name")"; then
    printf '%s' "$path"
    return 0
  fi
  echo "  community-tax: building ${pkg} wasm (artifact missing)"
  (
    cd "${REPO_ROOT}/smartcontracts" &&
      cargo build -p "$pkg" --release --target wasm32-unknown-unknown --offline 2>/dev/null \
      || cargo build -p "$pkg" --release --target wasm32-unknown-unknown
  )
  path="${REPO_ROOT}/smartcontracts/target/wasm32-unknown-unknown/release/${name}.wasm"
  if [ ! -f "$path" ]; then
    echo "ERROR: ${name}.wasm missing. Run make build-optimized." >&2
    return 1
  fi
  printf '%s' "$path"
}

_ctax_store() {
  local host="$1"
  local dest="/tmp/artifacts/$(basename "$host")"
  docker exec "$CONTAINER_NAME" mkdir -p /tmp/artifacts
  # docker cp can fail on this LocalTerra image (fuse-overlayfs mkdirat); stream bytes instead.
  docker exec -i "$CONTAINER_NAME" sh -c "cat > '$dest'" < "$host"
  local TX_HASH
  TX_HASH=$(terrad_tx wasm store "$dest" | jq -r '.txhash')
  echo "  store $(basename "$host"): $TX_HASH" >&2
  wait_tx "$TX_HASH"
  get_code_id "$TX_HASH"
}

_ctax_send_hook() {
  local token="$1" dest="$2" amount="$3" hook_json="$4"
  local b64 msg TX_HASH
  b64="$(printf '%s' "$hook_json" | base64 -w0 2>/dev/null || printf '%s' "$hook_json" | base64 | tr -d '\n')"
  msg="$(jq -nc --arg c "$dest" --arg amt "$amount" --arg m "$b64" \
    '{send:{contract:$c,amount:$amt,msg:$m}}')"
  TX_HASH=$(terrad_tx wasm execute "$token" "$msg" | jq -r '.txhash')
  wait_tx "$TX_HASH"
  printf '%s' "$TX_HASH"
}

_ctax_smart() {
  local addr="$1" q="$2"
  terrad_query wasm contract-state smart "$addr" "$q" | jq -c '.data'
}

# Sets (exported): COMMUNITY_TAX_CODE_ID, COMMUNITY_TOKEN_LAUNCHER,
# TOKEN_COMMUNITY_TAX_ADDRESS, PAIR_COMMUNITY_TAX_EMBER, COMMUNITY_TAX_UST1,
# COMMUNITY_TAX_AUTOLP. Empty when DEPLOY_SKIP_COMMUNITY_TAX=1.
deploy_community_tax_local() {
  COMMUNITY_TAX_CODE_ID=""
  COMMUNITY_TOKEN_LAUNCHER=""
  TOKEN_COMMUNITY_TAX_ADDRESS=""
  PAIR_COMMUNITY_TAX_EMBER=""
  COMMUNITY_TAX_UST1=""
  COMMUNITY_TAX_AUTOLP=""

  if [ "${DEPLOY_SKIP_COMMUNITY_TAX:-}" = "1" ]; then
    echo "[Phase 4d] Community tax skipped (DEPLOY_SKIP_COMMUNITY_TAX=1)"
    return 0
  fi

  local ember="${TOKEN_ADDRESSES[0]:-}"
  if [ -z "$FACTORY_ADDRESS" ] || [ -z "$ember" ]; then
    echo "ERROR: community-tax phase needs FACTORY_ADDRESS and EMBER (TOKEN_ADDRESSES[0])." >&2
    return 1
  fi

  echo ""
  echo "[Phase 4d] Community tax token + AutoLP + listed tax/EMBER (GitLab #620)"
  echo "----------------------------------------------"

  local TOKEN_WASM LAUNCHER_WASM AUTOLP_WASM
  TOKEN_WASM="$(_ctax_require_wasm cl8y_community_tax_token cl8y-community-tax-token)" || return 1
  LAUNCHER_WASM="$(_ctax_require_wasm cl8y_community_token_launcher cl8y-community-token-launcher)" || return 1
  AUTOLP_WASM="$(_ctax_require_wasm cl8y_community_tax_autolp cl8y-community-tax-autolp)" || return 1
  if [ ! -f "${ARTIFACTS_DIR}/cw20_mintable.wasm" ]; then
    echo "ERROR: ${ARTIFACTS_DIR}/cw20_mintable.wasm missing (SmokeUST1 stand-in)." >&2
    return 1
  fi

  local TOKEN_CODE LAUNCHER_CODE AUTOLP_CODE
  echo ""
  echo "[4d.1] Storing community-tax + launcher + AutoLP wasm (local ids only)…"
  TOKEN_CODE="$(_ctax_store "$TOKEN_WASM")"
  LAUNCHER_CODE="$(_ctax_store "$LAUNCHER_WASM")"
  AUTOLP_CODE="$(_ctax_store "$AUTOLP_WASM")"
  echo "  token=$TOKEN_CODE launcher=$LAUNCHER_CODE autolp=$AUTOLP_CODE"

  local pin
  for pin in "$TOKEN_CODE" "$LAUNCHER_CODE" "$AUTOLP_CODE"; do
    if _ctax_is_forbidden_code "$pin"; then
      echo "ERROR: LocalTerra store id $pin collides with a columbus-5 pin; refuse whitelist/store reuse (#620)." >&2
      return 1
    fi
  done

  local STAMP UST1_INIT UST1_TX UST1_ADDR
  STAMP="$(date +%s)"
  echo ""
  echo "[4d.2] Instantiating mintable SmokeUST1 (invoice stand-in; not columbus-5 UST1)…"
  UST1_INIT="$(jq -nc --arg a "$TEST_ADDRESS" \
    '{name:"SmokeUST1",symbol:"SUST",decimals:6,initial_balances:[{address:$a,amount:"1000000000000"}],mint:{minter:$a}}')"
  UST1_TX=$(terrad_tx wasm instantiate "$CW20_CODE_ID" "$UST1_INIT" \
    --label "620-ust1-${STAMP}" --admin "$TEST_ADDRESS" | jq -r '.txhash')
  wait_tx "$UST1_TX"
  UST1_ADDR="$(get_contract_address "$UST1_TX" | head -1 | tr -d '[:space:]')"
  echo "  UST1 stand-in: $UST1_ADDR"

  local LAUNCHER_INIT LAUNCHER_TX LAUNCHER_ADDR
  echo ""
  echo "[4d.3] Instantiating launcher (CMM stand-in=test1; autolp_code_id set)…"
  LAUNCHER_INIT="$(jq -nc \
    --argjson tok "$TOKEN_CODE" --argjson alp "$AUTOLP_CODE" \
    --arg ust1 "$UST1_ADDR" --arg a "$TEST_ADDRESS" \
    --arg factory "$FACTORY_ADDRESS" --arg router "${ROUTER_ADDRESS:-}" \
    '{
      token_code_id:$tok,
      autolp_code_id:$alp,
      ust1:$ust1,
      cmm_treasury:$a,
      cmm_governance:$a,
      factory:$factory,
      router: (if $router == "" then null else $router end)
    }')"
  LAUNCHER_TX=$(terrad_tx wasm instantiate "$LAUNCHER_CODE" "$LAUNCHER_INIT" \
    --label "620-launcher-${STAMP}" --admin "$TEST_ADDRESS" | jq -r '.txhash')
  wait_tx "$LAUNCHER_TX"
  LAUNCHER_ADDR="$(get_contract_address "$LAUNCHER_TX" | head -1 | tr -d '[:space:]')"
  echo "  launcher: $LAUNCHER_ADDR (LocalTerra CMM stand-in=$TEST_ADDRESS)"

  echo ""
  echo "[4d.4] AddWhitelistedCodeId local token $TOKEN_CODE only…"
  local already
  already="$(_ctax_smart "$FACTORY_ADDRESS" \
    "$(jq -nc --argjson c "$TOKEN_CODE" '{is_code_id_whitelisted:{code_id:$c}}')" \
    | jq -r '.whitelisted // false')"
  if [ "$already" = "true" ]; then
    echo "  local token code already whitelisted"
  else
    local WL_MSG WL_TX
    WL_MSG="$(jq -nc --argjson c "$TOKEN_CODE" '{add_whitelisted_code_id:{code_id:$c}}')"
    WL_TX=$(terrad_tx wasm execute "$FACTORY_ADDRESS" "$WL_MSG" | jq -r '.txhash')
    wait_tx "$WL_TX"
    echo "  whitelist tx: $WL_TX"
  fi

  local INVOICE=50000000
  local BUY_BPS=500
  local SELL_BPS=500
  local INIT_BAL="1000000000000000000"
  echo ""
  echo "[4d.5] Paid CreateToken features=[auto_v2_lp] (50 UST1 invoice → launcher)…"
  local CREATE_HOOK CREATE_TX TAX_TOKEN
  CREATE_HOOK="$(jq -nc --arg n "QATax" --arg s "QTAX" --arg a "$TEST_ADDRESS" \
    --argjson buy "$BUY_BPS" --argjson sell "$SELL_BPS" --arg bal "$INIT_BAL" \
    '{
      create_token:{
        name:$n, symbol:$s, decimals:6,
        initial_balances:[{address:$a,amount:$bal}],
        manager:$a, treasury:$a,
        buy_bps:$buy, sell_bps:$sell,
        max_buy_bps:$buy, max_sell_bps:$sell, max_transfer_bps:0,
        features:["auto_v2_lp"],
        mint:null, transfer_bps:null, sinks:null,
        launch_guards:null,
        autolp_threshold:null, autolp_lp_recipient:null
      }
    }')"
  CREATE_TX="$(_ctax_send_hook "$UST1_ADDR" "$LAUNCHER_ADDR" "$INVOICE" "$CREATE_HOOK")"
  TAX_TOKEN=""
  local cand
  while read -r cand; do
    [ "${cand#terra1}" = "$cand" ] && continue
    if _ctax_smart "$cand" '{"get_launcher_origin":{}}' 2>/dev/null \
      | jq -e --arg l "$LAUNCHER_ADDR" '.launcher == $l' >/dev/null 2>&1; then
      TAX_TOKEN="$cand"
      break
    fi
  done < <(terrad_query tx "$CREATE_TX" | terrad_jq_contract_address_from_tx_json)
  if [ "${TAX_TOKEN#terra1}" = "$TAX_TOKEN" ]; then
    echo "ERROR: paid CreateToken did not yield a launcher-origin token ($CREATE_TX)" >&2
    return 1
  fi
  echo "  QA tax token: $TAX_TOKEN"

  local ORIGIN ORIGIN_LAUNCHER FEAT AUTOLP
  ORIGIN="$(_ctax_smart "$TAX_TOKEN" '{"get_launcher_origin":{}}')"
  ORIGIN_LAUNCHER="$(echo "$ORIGIN" | jq -r '.launcher // empty')"
  if [ "$ORIGIN_LAUNCHER" != "$LAUNCHER_ADDR" ]; then
    echo "ERROR: GetLauncherOrigin.launcher=$ORIGIN_LAUNCHER != $LAUNCHER_ADDR" >&2
    return 1
  fi
  FEAT="$(_ctax_smart "$TAX_TOKEN" '{"get_features":{}}')"
  echo "$FEAT" | jq -e '.auto_v2_lp == true and .mint_control == false' >/dev/null || {
    echo "ERROR: QA token must have auto_v2_lp and no MintControl: $FEAT" >&2
    return 1
  }
  AUTOLP="$(echo "$(_ctax_smart "$TAX_TOKEN" '{"get_config":{}}')" | jq -r '.autolp // empty')"
  if [ "${AUTOLP#terra1}" = "$AUTOLP" ]; then
    echo "ERROR: AutoLP not bound on token GetConfig.autolp" >&2
    return 1
  fi
  echo "  AutoLP bound: $AUTOLP"

  local held_l held_t
  held_l="$(echo "$(_ctax_smart "$UST1_ADDR" "{\"balance\":{\"address\":\"$LAUNCHER_ADDR\"}}")" | jq -r '.balance // "0"')"
  held_t="$(echo "$(_ctax_smart "$UST1_ADDR" "{\"balance\":{\"address\":\"$TAX_TOKEN\"}}")" | jq -r '.balance // "0"')"
  if [ "$held_l" != "0" ] || [ "$held_t" != "0" ]; then
    echo "ERROR: launcher/token kept UST1 $held_l/$held_t (T592-4 / T606-6)" >&2
    return 1
  fi

  echo ""
  echo "[4d.6] CreatePair tax/EMBER + RegisterListedPair…"
  local PAIR_Q PAIR_EXISTING PAIR_ADDR CREATE_PAIR_MSG CREATE_PAIR_TX
  PAIR_Q="$(jq -nc --arg a "$TAX_TOKEN" --arg b "$ember" \
    '{pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
  PAIR_EXISTING="$( { _ctax_smart "$FACTORY_ADDRESS" "$PAIR_Q" || echo '{}'; } 2>/dev/null \
    | jq -r '.contract_addr // .pair.contract_addr // empty')"
  if [ "${PAIR_EXISTING#terra1}" != "$PAIR_EXISTING" ]; then
    echo "  pair already listed: $PAIR_EXISTING (idempotent skip CreatePair)"
    PAIR_ADDR="$PAIR_EXISTING"
  else
    CREATE_PAIR_MSG="$(jq -nc --arg a "$TAX_TOKEN" --arg b "$ember" \
      '{create_pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
    CREATE_PAIR_TX=$(factory_create_pair "$CREATE_PAIR_MSG" | jq -r '.txhash')
    wait_tx "$CREATE_PAIR_TX"
    PAIR_ADDR="$(get_contract_address "$CREATE_PAIR_TX" | head -1 | tr -d '[:space:]')"
    if [ "${PAIR_ADDR#terra1}" = "$PAIR_ADDR" ]; then
      PAIR_ADDR="$(_ctax_smart "$FACTORY_ADDRESS" "$PAIR_Q" | jq -r '.contract_addr // .pair.contract_addr // empty')"
    fi
  fi
  if [ "${PAIR_ADDR#terra1}" = "$PAIR_ADDR" ]; then
    echo "ERROR: tax/EMBER pair address missing" >&2
    return 1
  fi
  echo "  pair: $PAIR_ADDR"

  local REG_TX
  REG_TX=$(terrad_tx wasm execute "$TAX_TOKEN" \
    "$(jq -nc --arg p "$PAIR_ADDR" '{register_listed_pair:{pair:$p}}')" | jq -r '.txhash')
  wait_tx "$REG_TX"
  echo "  RegisterListedPair: $REG_TX"

  local LIQ="${COMMUNITY_TAX_SEED_LIQ:-100000000000}"
  echo ""
  echo "[4d.7] Seed LP ${LIQ}/${LIQ} (swarm floor ≥ 10M raw / side)…"
  local ALLOW_TX PROVIDE_TX CAND_BEFORE CAND_AFTER
  ALLOW_TX=$(terrad_tx wasm execute "$TAX_TOKEN" \
    "$(jq -nc --arg s "$PAIR_ADDR" --arg amt "$LIQ" \
      '{increase_allowance:{spender:$s,amount:$amt,expires:{never:{}}}}')" | jq -r '.txhash')
  wait_tx "$ALLOW_TX"
  ALLOW_TX=$(terrad_tx wasm execute "$ember" \
    "$(jq -nc --arg s "$PAIR_ADDR" --arg amt "$LIQ" \
      '{increase_allowance:{spender:$s,amount:$amt,expires:{never:{}}}}')" | jq -r '.txhash')
  wait_tx "$ALLOW_TX"
  CAND_BEFORE="$(echo "$(_ctax_smart "$TAX_TOKEN" "{\"balance\":{\"address\":\"$PAIR_ADDR\"}}")" | jq -r '.balance // "0"')"
  PROVIDE_TX=$(terrad_tx wasm execute "$PAIR_ADDR" \
    "$(jq -nc --arg a "$TAX_TOKEN" --arg b "$ember" --arg amt "$LIQ" \
      '{provide_liquidity:{assets:[{info:{token:{contract_addr:$a}},amount:$amt},{info:{token:{contract_addr:$b}},amount:$amt}],slippage_tolerance:null,receiver:null,deadline:null}}')" \
    | jq -r '.txhash')
  wait_tx "$PROVIDE_TX"
  CAND_AFTER="$(echo "$(_ctax_smart "$TAX_TOKEN" "{\"balance\":{\"address\":\"$PAIR_ADDR\"}}")" | jq -r '.balance // "0"')"
  python3 -c '
import sys
b, a, amt = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
if a - b != amt:
    sys.stderr.write(f"ERROR: provide pair credit {b}->{a} != {amt} (T592-1)\n")
    sys.exit(1)
print(f"  provide 1:1 pair {b}->{a}")
' "$CAND_BEFORE" "$CAND_AFTER" "$LIQ"

  echo ""
  echo "[4d.8] AutoLP UpdateConfig { pair } (factory-listed tax pair)…"
  local UPD_TX ALP_PAIR
  UPD_TX=$(terrad_tx wasm execute "$AUTOLP" \
    "$(jq -nc --arg p "$PAIR_ADDR" '{update_config:{pair:$p}}')" | jq -r '.txhash')
  wait_tx "$UPD_TX"
  ALP_PAIR="$(echo "$(_ctax_smart "$AUTOLP" '{"get_config":{}}')" | jq -r '.pair // empty')"
  if [ "$ALP_PAIR" != "$PAIR_ADDR" ]; then
    echo "ERROR: AutoLP GetConfig.pair=$ALP_PAIR != $PAIR_ADDR (M610-1)" >&2
    return 1
  fi
  echo "  AutoLP pair=$ALP_PAIR"

  if [ "${COMMUNITY_TAX_SKIP_SKIM:-}" != "1" ]; then
    echo ""
    echo "[4d.9] Optional AutoLP seed Transfer + SkimToLp (permissionless; not from token Send)…"
    local SKIM_SEED="${COMMUNITY_TAX_SKIM_SEED:-1000000}"
    local SKIM_TX
    SKIM_TX=$(terrad_tx wasm execute "$TAX_TOKEN" \
      "$(jq -nc --arg r "$AUTOLP" --arg amt "$SKIM_SEED" '{transfer:{recipient:$r,amount:$amt}}')" | jq -r '.txhash')
    wait_tx "$SKIM_TX"
    set +e
    SKIM_TX=$(terrad_tx wasm execute "$AUTOLP" '{"skim_to_lp":{}}' 2>/dev/null | jq -r '.txhash')
    local skim_st=$?
    set -e
    if [ "$skim_st" -eq 0 ] && [ -n "$SKIM_TX" ] && [ "$SKIM_TX" != "null" ]; then
      wait_tx "$SKIM_TX" || true
      echo "  SkimToLp tx: $SKIM_TX"
    else
      echo "  SkimToLp skipped or reverted (floor / thin book is OK — M610-3/4)"
    fi
  fi

  COMMUNITY_TAX_CODE_ID="$TOKEN_CODE"
  COMMUNITY_TOKEN_LAUNCHER="$LAUNCHER_ADDR"
  TOKEN_COMMUNITY_TAX_ADDRESS="$TAX_TOKEN"
  PAIR_COMMUNITY_TAX_EMBER="$PAIR_ADDR"
  COMMUNITY_TAX_UST1="$UST1_ADDR"
  COMMUNITY_TAX_AUTOLP="$AUTOLP"
  export COMMUNITY_TAX_CODE_ID COMMUNITY_TOKEN_LAUNCHER TOKEN_COMMUNITY_TAX_ADDRESS \
    PAIR_COMMUNITY_TAX_EMBER COMMUNITY_TAX_UST1 COMMUNITY_TAX_AUTOLP
  echo ""
  echo "  Community tax seed complete (local code $TOKEN_CODE; not 11611)."
}
