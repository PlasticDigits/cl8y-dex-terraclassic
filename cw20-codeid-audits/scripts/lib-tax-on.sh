#!/usr/bin/env bash
# Shared LocalTerra helpers for community-tax **tax-on** paths (GitLab #623).
#
# Sourced by layer-b-tax-on.sh and the #601 smoke so store / pair / preview /
# sell-buy asserts do not diverge. Requires lib-layer-lt.sh + e2e-terrad-tx
# already sourced. Never run against columbus-5.
#
# Do **not** source this from layer-b-lt.sh — B-lt stays tax-off 1:1 (C623-1).
# shellcheck shell=bash

TAX_ON_FORBIDDEN_CODE_IDS="11611 11612 11613 11614 11619 11620 11621 11622 8654"
TAX_ON_SELL_BPS="${TAX_ON_SELL_BPS:-500}"
TAX_ON_BUY_BPS="${TAX_ON_BUY_BPS:-500}"
TAX_ON_PROVIDE_RAW="${TAX_ON_PROVIDE_RAW:-${VERIFY601_PROVIDE_RAW:-100000000}}"
TAX_ON_SWAP_RAW="${TAX_ON_SWAP_RAW:-${VERIFY601_SWAP_RAW:-1000000}}"
TAX_ON_LIMIT_RAW="${TAX_ON_LIMIT_RAW:-${LAYER_LT_LIMIT_RAW:-1000000}}"
TAX_ON_SKIM_RAW="${TAX_ON_SKIM_RAW:-1000000}"

tax_on_is_forbidden_code() {
  local id="$1" x
  for x in $TAX_ON_FORBIDDEN_CODE_IDS; do
    if [[ "$id" == "$x" ]]; then
      return 0
    fi
  done
  return 1
}

# Worktrees often lack artifacts/; fall back to the primary clone, then cargo.
tax_on_find_wasm() {
  local name="$1"
  local pkg="$2"
  local cand primary
  primary="$(cd "$REPO_ROOT/.." 2>/dev/null && pwd)/cl8y-dex-terraclassic"
  for cand in \
    "$REPO_ROOT/smartcontracts/artifacts/${name}.wasm" \
    "$primary/smartcontracts/artifacts/${name}.wasm" \
    "$REPO_ROOT/smartcontracts/target/wasm32-unknown-unknown/release/${name}.wasm" \
    "$primary/smartcontracts/target/wasm32-unknown-unknown/release/${name}.wasm"; do
    if [[ -f "$cand" ]]; then
      printf '%s' "$cand"
      return 0
    fi
  done
  echo "tax-on: building ${pkg} wasm (artifact missing)" >&2
  (
    cd "$REPO_ROOT/smartcontracts" &&
      cargo build -p "$pkg" --release --target wasm32-unknown-unknown --offline 2>/dev/null \
      || cargo build -p "$pkg" --release --target wasm32-unknown-unknown
  )
  cand="$REPO_ROOT/smartcontracts/target/wasm32-unknown-unknown/release/${name}.wasm"
  [[ -f "$cand" ]] || {
    echo "FAIL: ${name}.wasm missing. Run make build-optimized." >&2
    return 1
  }
  printf '%s' "$cand"
}

tax_on_terrad_tx() {
  e2e_terrad_tx "$CONTAINER" "$@"
}

tax_on_exec_ok() {
  local out tx
  out="$(tax_on_terrad_tx "$@")"
  tx="$(layer_txhash "$out")"
  [[ -n "$tx" ]] || {
    echo "FAIL: no txhash for: $*" >&2
    printf '%s\n' "$out" >&2
    exit 1
  }
  layer_wait_tx "$tx"
  printf '%s' "$tx"
}

tax_on_stream_to_container() {
  # docker cp can fail on this LocalTerra image (fuse-overlayfs mkdirat).
  local host="$1" dest="$2"
  if [[ -e "$host" ]]; then
    host="$(readlink -f "$host")"
  fi
  if [[ -n "${LOCALTERRA_DOCKER_VIA_SG:-}" ]] && command -v sg >/dev/null 2>&1; then
    sg docker -c "docker exec -i $(printf '%q' "$CONTAINER") sh -c $(printf '%q' "cat > '$dest'")" < "$host"
  else
    docker exec -i "$CONTAINER" sh -c "cat > '$dest'" < "$host"
  fi
}

tax_on_store_wasm() {
  local host="$1" dest="$2"
  tax_on_stream_to_container "$host" "$dest"
  local out tx json code
  out="$(tax_on_terrad_tx wasm store "$dest")"
  tx="$(layer_txhash "$out")"
  [[ -n "$tx" ]] || {
    echo "FAIL: store $host produced no txhash" >&2
    printf '%s\n' "$out" >&2
    exit 1
  }
  json="$(terrad_wait_tx_query "$CONTAINER" "$tx" "$TERRAD_NODE")"
  code="$(echo "$json" | terrad_jq_code_id_from_tx_json | head -1 | tr -d '[:space:]')"
  [[ "$code" =~ ^[0-9]+$ ]] || {
    echo "FAIL: could not parse code_id from store $tx" >&2
    exit 1
  }
  if tax_on_is_forbidden_code "$code"; then
    echo "FAIL: local store id $code collides with a columbus-5 pin; refuse reuse (C623-5)." >&2
    exit 1
  fi
  printf '%s' "$code"
}

tax_on_contract_from_tx() {
  local tx="$1"
  echo "$(terrad_wait_tx_query "$CONTAINER" "$tx" "$TERRAD_NODE")" \
    | terrad_jq_contract_address_from_tx_json | head -1
}

tax_on_send_cw20_hook() {
  local token="$1" dest="$2" amount="$3" hook_json="$4"
  local b64 msg
  b64="$(printf '%s' "$hook_json" | base64 -w0 2>/dev/null || printf '%s' "$hook_json" | base64 | tr -d '\n')"
  msg="$(jq -nc --arg c "$dest" --arg amt "$amount" --arg m "$b64" \
    '{send:{contract:$c,amount:$amt,msg:$m}}')"
  tax_on_exec_ok wasm execute "$token" "$msg"
}

tax_on_send_cw20_hook_from() {
  local from_key="$1" token="$2" dest="$3" amount="$4" hook_json="$5"
  local b64 msg out tx
  b64="$(printf '%s' "$hook_json" | base64 -w0 2>/dev/null || printf '%s' "$hook_json" | base64 | tr -d '\n')"
  msg="$(jq -nc --arg c "$dest" --arg amt "$amount" --arg m "$b64" \
    '{send:{contract:$c,amount:$amt,msg:$m}}')"
  out="$(layer_terrad_tx_from "$from_key" wasm execute "$token" "$msg")"
  tx="$(layer_txhash "$out")"
  [[ -n "$tx" ]] || {
    echo "FAIL: no txhash for $from_key send hook" >&2
    printf '%s\n' "$out" >&2
    exit 1
  }
  layer_wait_tx "$tx"
  printf '%s' "$tx"
}

tax_on_try_tx() {
  # Always returns 0 so command-substitution stays set -e safe. Caller uses
  # layer_execute_rejected on the printed output.
  local out
  set +e
  out="$(tax_on_terrad_tx "$@" 2>&1)"
  set -e
  printf '%s\n' "$out"
  return 0
}

tax_on_try_tx_from() {
  local from_key="$1"
  shift
  local out
  set +e
  out="$(layer_terrad_tx_from "$from_key" "$@" 2>&1)"
  set -e
  printf '%s\n' "$out"
  return 0
}

tax_on_lcd_admin() {
  local addr="$1"
  local raw
  raw="$(localterra_lcd_curl "$LCD" "/cosmwasm/wasm/v1/contract/${addr}")"
  echo "$raw" | jq -r '.contract_info.admin // empty'
}

tax_on_is_whitelisted() {
  local factory="$1" code="$2"
  local raw
  raw="$(layer_smart "$factory" \
    "$(jq -nc --argjson c "$code" '{is_code_id_whitelisted:{code_id:$c}}')" 2>/dev/null || true)"
  if echo "$raw" | jq -e '.whitelisted == true' >/dev/null 2>&1; then
    return 0
  fi
  raw="$(layer_smart "$factory" '{"get_whitelisted_code_ids":{}}')"
  echo "$raw" | jq -e --argjson c "$code" '.code_ids | index($c) != null' >/dev/null 2>&1
}

tax_on_whitelist_local() {
  local factory="$1" code="$2"
  tax_on_is_forbidden_code "$code" && {
    echo "FAIL: refusing to whitelist columbus-5 pin $code (C623-5)." >&2
    exit 1
  }
  if tax_on_is_whitelisted "$factory" "$code"; then
    echo "tax-on: local code $code already whitelisted"
    return 0
  fi
  local msg
  msg="$(jq -nc --argjson c "$code" '{add_whitelisted_code_id:{code_id:$c}}')"
  tax_on_exec_ok wasm execute "$factory" "$msg" >/dev/null
  tax_on_is_whitelisted "$factory" "$code" || {
    echo "FAIL: local code $code not whitelisted after AddWhitelistedCodeId" >&2
    exit 1
  }
}

tax_on_factory_pair() {
  local factory="$1" a="$2" b="$3"
  local q raw
  q="$(jq -nc --arg a "$a" --arg b "$b" \
    '{pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
  raw="$(lcd_smart_query_raw "$LCD" "$factory" "$q" 2>/dev/null || true)"
  lcd_decode_smart_data "$raw" 2>/dev/null | jq -r '.contract_addr // .pair.contract_addr // empty' || true
}

tax_on_resolve_pair() {
  local factory="$1" a="$2" b="$3"
  local addr
  addr="$(tax_on_factory_pair "$factory" "$a" "$b")"
  if [[ "$addr" != terra1* ]]; then
    addr="$(tax_on_factory_pair "$factory" "$b" "$a")"
  fi
  printf '%s' "$addr"
}

tax_on_create_pair() {
  local factory="$1" a="$2" b="$3"
  local existing
  existing="$(tax_on_resolve_pair "$factory" "$a" "$b")"
  if [[ "$existing" == terra1* ]]; then
    printf '%s' "$existing"
    return 0
  fi
  local cfg fee create_msg fee_args=() tx addr
  cfg="$(layer_smart "$factory" '{"config":{}}')"
  fee="$(echo "$cfg" | jq -r '.pair_creation_fee_uluna // "0"')"
  create_msg="$(jq -nc --arg a "$a" --arg b "$b" \
    '{create_pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')"
  if [[ "$fee" != "0" && -n "$fee" ]]; then
    fee_args=(--amount "${fee}uluna")
  fi
  tx="$(tax_on_exec_ok wasm execute "$factory" "$create_msg" "${fee_args[@]}")"
  addr="$(tax_on_contract_from_tx "$tx")"
  if [[ "$addr" != terra1* ]]; then
    addr="$(tax_on_resolve_pair "$factory" "$a" "$b")"
  fi
  [[ "$addr" == terra1* ]] || {
    echo "FAIL: CreatePair did not resolve pair address" >&2
    exit 1
  }
  printf '%s' "$addr"
}

tax_on_is_listed_pair() {
  local token="$1" pair="$2"
  local raw
  raw="$(layer_smart "$token" \
    "$(jq -nc --arg p "$pair" '{is_protocol_exempt:{address:$p}}')" 2>/dev/null || true)"
  echo "$raw" | jq -e '.protocol == true' >/dev/null 2>&1
}

tax_on_register_listed() {
  local token="$1" pair="$2"
  if tax_on_is_listed_pair "$token" "$pair"; then
    echo "tax-on: pair $pair already registered"
    return 0
  fi
  local msg
  msg="$(jq -nc --arg p "$pair" '{register_listed_pair:{pair:$p}}')"
  tax_on_exec_ok wasm execute "$token" "$msg" >/dev/null
}

tax_on_provide_1to1() {
  local token="$1" quote="$2" pair="$3" amt="$4"
  local allow cand_before cand_after provide
  allow="$(jq -nc --arg s "$pair" --arg amt "$amt" \
    '{increase_allowance:{spender:$s,amount:$amt}}')"
  tax_on_exec_ok wasm execute "$token" "$allow" >/dev/null
  tax_on_exec_ok wasm execute "$quote" "$allow" >/dev/null
  cand_before="$(layer_cw20_balance "$token" "$pair")"
  provide="$(jq -nc --arg a "$token" --arg b "$quote" --arg amt "$amt" \
    '{provide_liquidity:{assets:[{info:{token:{contract_addr:$a}},amount:$amt},{info:{token:{contract_addr:$b}},amount:$amt}],slippage_tolerance:null,receiver:null,deadline:null}}')"
  tax_on_exec_ok wasm execute "$pair" "$provide" >/dev/null
  cand_after="$(layer_cw20_balance_changed "$token" "$pair" "$cand_before")"
  python3 -c '
import sys
b, a, amt = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
if a - b != amt:
    sys.stderr.write(f"FAIL: provide pair credit {b}->{a} != {amt} (T592-1 / P2)\n")
    sys.exit(1)
print(f"tax-on: provide TransferFrom 1:1 ({amt})")
' "$cand_before" "$cand_after" "$amt"
}

tax_on_preview_sell() {
  local token="$1" trader="$2" pair="$3" amt="$4" sell_bps="$5"
  local hook b64 preview pred_debit pred_credit expect_tax expect_debit
  hook='{"swap":{"max_spread":"1"}}'
  b64="$(printf '%s' "$hook" | base64 -w0 2>/dev/null || printf '%s' "$hook" | base64 | tr -d '\n')"
  preview="$(layer_smart "$token" \
    "$(jq -nc --arg f "$trader" --arg t "$pair" --arg amt "$amt" --arg m "$b64" \
      '{tax_preview:{from:$f,to:$t,amount:$amt,send_msg:$m}}')")"
  pred_debit="$(echo "$preview" | jq -r '.debit')"
  pred_credit="$(echo "$preview" | jq -r '.credit')"
  expect_tax=$((amt * sell_bps / 10000))
  expect_debit=$((amt + expect_tax))
  [[ "$pred_debit" == "$expect_debit" && "$pred_credit" == "$amt" ]] || {
    echo "FAIL: TaxPreview sell debit=$pred_debit credit=$pred_credit (want debit=$expect_debit credit=$amt)" >&2
    echo "$preview" >&2
    exit 1
  }
  echo "$expect_tax"
}

tax_on_assert_sell() {
  local user_b="$1" user_a="$2" pair_b="$3" pair_a="$4" sink_b="$5" sink_a="$6" amt="$7" tax="$8"
  python3 -c '
import sys
ub, ua, pb, pa, sb, sa, amt, tax = (int(x) for x in sys.argv[1:])
if ub - ua != amt + tax:
    sys.stderr.write(f"FAIL: sell user debit {ub}->{ua} != {amt}+{tax} (T592-2)\n")
    sys.exit(1)
if pa - pb != amt:
    sys.stderr.write(f"FAIL: sell pair credit {pb}->{pa} != {amt} (H-01 inbound 1:1)\n")
    sys.exit(1)
if sa - sb != tax:
    sys.stderr.write(f"FAIL: sell treasury {sb}->{sa} != {tax}\n")
    sys.exit(1)
print("tax-on: sell extra-debit + pair inbound 1:1")
' "$user_b" "$user_a" "$pair_b" "$pair_a" "$sink_b" "$sink_a" "$amt" "$tax"
}

tax_on_assert_buy_split() {
  local user_b="$1" user_a="$2" pair_b="$3" pair_a="$4" buy_bps="$5"
  python3 -c '
import sys
ub, ua, pb, pa, bps = (int(x) for x in sys.argv[1:])
pair_debit = pb - pa
user_credit = ua - ub
if pair_debit <= 0 or user_credit <= 0:
    sys.stderr.write(f"FAIL: buy did not move tax token pair {pb}->{pa} user {ub}->{ua}\n")
    sys.exit(1)
if user_credit >= pair_debit:
    sys.stderr.write(f"FAIL: buy user credit {user_credit} >= pair debit {pair_debit} (expected outbound split)\n")
    sys.exit(1)
tax = pair_debit - user_credit
expect = pair_debit * bps // 10000
if abs(tax - expect) > 1:
    sys.stderr.write(f"FAIL: buy tax {tax} != ~{expect} ({bps} bps of {pair_debit})\n")
    sys.exit(1)
print(f"tax-on: buy outbound split pair_debit={pair_debit} user={user_credit} tax={tax}")
' "$user_b" "$user_a" "$pair_b" "$pair_a" "$buy_bps"
}

tax_on_assert_limit_1to1() {
  local ub="$1" ua="$2" pb="$3" pa="$4" tb="$5" ta="$6" amt="$7"
  local sb="${8:-0}" sa="${9:-0}" trader_is_sink="${10:-0}"
  python3 -c '
import sys
ub, ua, pb, pa, tb, ta, amt, sb, sa = (int(x) for x in sys.argv[1:10])
trader_is_sink = sys.argv[10]
debit = ub - ua
credited = (pa - pb) + (ta - tb)
if trader_is_sink != "1":
    credited += sa - sb
if debit <= 0:
    sys.stderr.write(f"FAIL: limit Send user debit {ub}->{ua} not positive\n")
    sys.exit(1)
if debit > amt:
    sys.stderr.write(f"FAIL: tax-on limit extra-debited Place {debit} > declared {amt}\n")
    sys.exit(1)
if credited != debit:
    sys.stderr.write(
        f"FAIL: limit escrow pair+fee+sink credit {credited} != user debit {debit} (L1)\n"
    )
    sys.exit(1)
print(f"tax-on: limit Place 1:1 debit={debit} (no sell extra-debit)")
' "$ub" "$ua" "$pb" "$pa" "$tb" "$ta" "$amt" "$sb" "$sa" "$trader_is_sink"
}

tax_on_token_init_msg() {
  local name="$1" symbol="$2" manager="$3" treasury="$4" factory="$5" router="$6" ust1="$7"
  local buy="$8" sell="$9"
  jq -nc \
    --arg n "$name" --arg s "$symbol" --arg a "$manager" --arg treas "$treasury" \
    --arg factory "$factory" --arg router "$router" --arg ust1 "$ust1" \
    --argjson buy "$buy" --argjson sell "$sell" \
    '{
      name:$n, symbol:$s, decimals:6,
      initial_balances:[{address:$a,amount:"1000000000000"}],
      marketing:{},
      manager:$a, treasury:$treas,
      buy_bps:$buy, sell_bps:$sell,
      max_buy_bps:$buy, max_sell_bps:$sell, max_transfer_bps:0,
      factory:$factory,
      router: (if $router == "" then null else $router end),
      ust1:$ust1,
      cmm_treasury:$a,
      features:[],
      mint:null,
      transfer_bps:null,
      sinks:null,
      autolp:null,
      launcher:null,
      launch_guards:null,
      initial_exempt:null
    }'
}

tax_on_read_bps() {
  local token="$1"
  local cfg
  cfg="$(layer_smart "$token" '{"get_config":{}}')"
  echo "$cfg" | jq -r '"\(.buy_bps) \(.sell_bps) \(.treasury // "") \(.router // "") \(.autolp // "")"'
}
