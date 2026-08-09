#!/usr/bin/env bash
# UST1 secondary AMM pair defaults (GitLab #508 / parent #502 Phase 4).
#
# Invariants (see docs/runbooks/ust1-secondary-amm-pair.md):
#   U1 — AMM is secondary; /ust1 oracle window remains primary mint/redeem.
#   U2 — Prefer CW20 code 10184 tokens already on factory whitelist (SL1).
#   U3 — Pair assets are CW20 only (UST1/vFDUSD and/or UST1/cUSTC) — never native uusd/uluna.
#   U4 — Seed liquidity is smoke/discovery sized; do not market as peg defense vs window.
#   U5 — Factory-provenance only (indexer P1); do not inject foreign pairs.
#   U6 — Do not fold UST1 into soft-launch gemstone defaults (SL5 / mainnet-soft-launch-defaults.sh).
#   U7 — Path A records pair addr + create/seed txs; Path B records explicit product waiver.
#
# shellcheck shell=bash

# Production factory / fee-discount (columbus-5 soft-launch anchors).
UST1_SEC_FACTORY_ADDRESS="${UST1_SEC_FACTORY_ADDRESS:-terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea}"
UST1_SEC_ROUTER_ADDRESS="${UST1_SEC_ROUTER_ADDRESS:-terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw}"
UST1_SEC_FEE_DISCOUNT_ADDRESS="${UST1_SEC_FEE_DISCOUNT_ADDRESS:-terra1wcczsdk7jwj99n3my6wx8wr4ee0hn6yaapgd792lgx5elrdtrn2scfnecz}"

# Phase 2 / 3 token anchors (issue #508 Known mainnet anchors).
UST1_SEC_UST1_ADDRESS="${UST1_SEC_UST1_ADDRESS:-terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72}"
UST1_SEC_VFDUSD_ADDRESS="${UST1_SEC_VFDUSD_ADDRESS:-terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3}"
UST1_SEC_CUSTC_ADDRESS="${UST1_SEC_CUSTC_ADDRESS:-terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch}"

# Expected CW20 code ID on columbus-5 (already SL1-whitelisted).
UST1_SEC_EXPECTED_CW20_CODE_ID="${UST1_SEC_EXPECTED_CW20_CODE_ID:-10184}"

# Preferred leg: vfdusd (closest to window) | custc
UST1_SEC_PAIR_LEG="${UST1_SEC_PAIR_LEG:-vfdusd}"

# Smoke/discovery seed (raw 6-decimal units). Override when inventory allows larger size.
# Default 1_000_000 = 1.0 token per side — discovery only, not peg defense (U4).
UST1_SEC_SEED_AMOUNT_A="${UST1_SEC_SEED_AMOUNT_A:-1000000}"
UST1_SEC_SEED_AMOUNT_B="${UST1_SEC_SEED_AMOUNT_B:-1000000}"

# Deployer key for host terrad (same soft-launch hot wallet by default).
UST1_SEC_DEPLOY_KEY="${UST1_SEC_DEPLOY_KEY:-cl8ydeploy}"
UST1_SEC_DEPLOY_ADDR="${UST1_SEC_DEPLOY_ADDR:-terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv}"

# Output dir for addresses / traces (not soft-launch catalog).
UST1_SEC_DEPLOY_DIR_REL="${UST1_SEC_DEPLOY_DIR_REL:-deployments/ust1-secondary-pair}"

ust1_sec_quote_address() {
  case "${UST1_SEC_PAIR_LEG}" in
    vfdusd | VFDUSD) printf '%s' "$UST1_SEC_VFDUSD_ADDRESS" ;;
    custc | cUSTC | CUSTC) printf '%s' "$UST1_SEC_CUSTC_ADDRESS" ;;
    *)
      echo "ust1-secondary-pair: unknown UST1_SEC_PAIR_LEG=${UST1_SEC_PAIR_LEG} (use vfdusd|custc)" >&2
      return 1
      ;;
  esac
}

ust1_sec_quote_symbol() {
  case "${UST1_SEC_PAIR_LEG}" in
    vfdusd | VFDUSD) printf 'vFDUSD' ;;
    custc | cUSTC | CUSTC) printf 'cUSTC' ;;
    *) return 1 ;;
  esac
}

ust1_sec_pair_label() {
  printf 'UST1/%s' "$(ust1_sec_quote_symbol)"
}
