#!/usr/bin/env bash
# Soft-launch (non-economic) CW20 + pair defaults for columbus-5.
#
# Invariants (see docs/runbooks/mainnet-soft-launch.md):
#   SL1 — Factory whitelisted_code_ids contains ONLY cw20-base (6036) + cw20-mintable (10184).
#   SL2 — No Terraport/GDEX/economic CW20 code IDs on the whitelist for this launch path.
#   SL3 — Trading tokens are 6 decimals; fee-discount uses mainnet CL8Y (18 decimals).
#   SL4 — Deploy key pays gas + bootstraps admin msgs; wasm --admin and final
#         config.governance = GOVERNANCE_MULTISIG_ADDR. Factory treasury is the
#         ustr-cmm CMM (MAINNET_CMM_TREASURY), not the multisig.
#   SL5 — Wrap-mapper / USTR treasury contracts are out of scope (CW20-only pairs).
#         Post-SL5 wrap is Coolify-only (#507) — do NOT add wrap addresses to this catalog.
#
# shellcheck shell=bash

# Deployer (hot wallet) — pays gas + instantiates; not governance.
MAINNET_SOFT_LAUNCH_DEPLOY_KEY="${MAINNET_SOFT_LAUNCH_DEPLOY_KEY:-cl8ydeploy}"
MAINNET_SOFT_LAUNCH_DEPLOY_ADDR="${MAINNET_SOFT_LAUNCH_DEPLOY_ADDR:-terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv}"

# Production governance / wasm admin (multisig). Fee treasury is CMM, not the multisig.
# shellcheck source=governance-multisig.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/governance-multisig.sh"
MAINNET_SOFT_LAUNCH_GOVERNANCE="${MAINNET_SOFT_LAUNCH_GOVERNANCE:-$GOVERNANCE_MULTISIG_ADDR}"
MAINNET_CMM_TREASURY="${MAINNET_CMM_TREASURY:-terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2}"
MAINNET_SOFT_LAUNCH_TREASURY="${MAINNET_SOFT_LAUNCH_TREASURY:-$MAINNET_CMM_TREASURY}"

# Mainnet CL8Y CW20 (fee-discount cl8y_token).
MAINNET_CL8Y_TOKEN_ADDRESS="${MAINNET_CL8Y_TOKEN_ADDRESS:-terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3}"

# CW20 mintable already stored on columbus-5 (PlasticDigits/cw20-mintable README).
# Override to force a fresh store of artifacts/cw20_mintable.wasm.
MAINNET_CW20_MINTABLE_CODE_ID="${MAINNET_CW20_MINTABLE_CODE_ID:-10184}"

# Terraswap columbus-5 cw20-base (code 6036; same template as GRDX etc.).
# Override empty / FORCE_STORE path: unset and ensure CW20_BASE_WASM, or set FORCE via deploy script store branch.
MAINNET_CW20_BASE_CODE_ID="${MAINNET_CW20_BASE_CODE_ID:-6036}"

# Factory defaults (bps). Pair creation fee matches on-chain default (100 LUNC).
MAINNET_SOFT_LAUNCH_DEFAULT_FEE_BPS="${MAINNET_SOFT_LAUNCH_DEFAULT_FEE_BPS:-180}"
MAINNET_SOFT_LAUNCH_PAIR_CREATION_FEE_ULUNA="${MAINNET_SOFT_LAUNCH_PAIR_CREATION_FEE_ULUNA:-100000000}"

# Production frontend / indexer hostnames (Coolify).
MAINNET_SOFT_LAUNCH_FRONTEND_ORIGIN="${MAINNET_SOFT_LAUNCH_FRONTEND_ORIGIN:-https://dex.cl8y.com}"
MAINNET_SOFT_LAUNCH_INDEXER_ORIGIN="${MAINNET_SOFT_LAUNCH_INDEXER_ORIGIN:-https://indexer.dex.cl8y.com}"

# Token catalog: name|symbol|decimals|kind|initial_amount
# kind=mintable → cw20-mintable (minter = deploy addr)
# kind=base     → cw20-base fixed supply (no mint after instantiate)
#
# Non-economic gemstone set — variety for routing / UI without economic TVL.
MAINNET_SOFT_LAUNCH_TOKENS=(
  "Ember|EMBER|6|mintable|10000000000000"
  "Coral|CORAL|6|mintable|10000000000000"
  "Jade|JADE|6|mintable|10000000000000"
  "Onyx|ONYX|6|mintable|10000000000000"
  "Ruby|RUBY|6|mintable|10000000000000"
  "Topaz|TOPAZ|6|mintable|10000000000000"
  "Quartz|QUARTZ|6|base|5000000000000"
  "Pearl|PEARL|6|base|5000000000000"
)

# Pair catalog: symbol_a|symbol_b|amount_a|amount_b (raw 6-decimal units)
MAINNET_SOFT_LAUNCH_PAIRS=(
  "EMBER|CORAL|100000000000|100000000000"
  "EMBER|JADE|1000000000|100000000000"
  "EMBER|ONYX|2000000000|100000000000"
  "CORAL|RUBY|50000000000|1000000000"
  "JADE|TOPAZ|100000000000|100000000000"
  "ONYX|QUARTZ|100000000000|50000000000"
  "RUBY|PEARL|1000000000|50000000000"
  "EMBER|QUARTZ|50000000000|50000000000"
  "CORAL|PEARL|100000000000|50000000000"
  "JADE|ONYX|100000000000|100000000000"
)

# Fee-discount add_tier payloads (must stay aligned with docs/reference/fee-discount-tiers.md).
# Parsed by scripts/check_fee_discount_tier_docs.py.
mainnet_soft_launch_fee_discount_tier_msgs() {
  cat <<'EOF'
{"add_tier":{"tier_id":0,"min_cl8y_balance":"0","discount_bps":10000,"limit_discount_bps":10000,"governance_only":true}}
{"add_tier":{"tier_id":1,"min_cl8y_balance":"1000000000000000000","discount_bps":250,"limit_discount_bps":1000,"governance_only":false}}
{"add_tier":{"tier_id":2,"min_cl8y_balance":"5000000000000000000","discount_bps":1000,"limit_discount_bps":2000,"governance_only":false}}
{"add_tier":{"tier_id":3,"min_cl8y_balance":"20000000000000000000","discount_bps":2000,"limit_discount_bps":3500,"governance_only":false}}
{"add_tier":{"tier_id":4,"min_cl8y_balance":"75000000000000000000","discount_bps":3500,"limit_discount_bps":5000,"governance_only":false}}
{"add_tier":{"tier_id":5,"min_cl8y_balance":"200000000000000000000","discount_bps":5000,"limit_discount_bps":6000,"governance_only":false}}
{"add_tier":{"tier_id":6,"min_cl8y_balance":"500000000000000000000","discount_bps":6000,"limit_discount_bps":7500,"governance_only":false}}
{"add_tier":{"tier_id":7,"min_cl8y_balance":"1500000000000000000000","discount_bps":7500,"limit_discount_bps":8500,"governance_only":false}}
{"add_tier":{"tier_id":8,"min_cl8y_balance":"3500000000000000000000","discount_bps":8500,"limit_discount_bps":9500,"governance_only":false}}
{"add_tier":{"tier_id":9,"min_cl8y_balance":"7500000000000000000000","discount_bps":9500,"limit_discount_bps":10000,"governance_only":false}}
{"add_tier":{"tier_id":255,"min_cl8y_balance":"0","discount_bps":0,"limit_discount_bps":0,"governance_only":true}}
EOF
}

mainnet_soft_launch_token_count() {
  echo "${#MAINNET_SOFT_LAUNCH_TOKENS[@]}"
}

mainnet_soft_launch_pair_count() {
  echo "${#MAINNET_SOFT_LAUNCH_PAIRS[@]}"
}

mainnet_soft_launch_symbol_index() {
  local want="$1"
  local i entry sym
  for i in "${!MAINNET_SOFT_LAUNCH_TOKENS[@]}"; do
    entry="${MAINNET_SOFT_LAUNCH_TOKENS[$i]}"
    sym="$(echo "$entry" | cut -d'|' -f2)"
    if [[ "$sym" == "$want" ]]; then
      echo "$i"
      return 0
    fi
  done
  return 1
}
