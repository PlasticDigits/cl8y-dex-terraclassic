#!/usr/bin/env bash
# Columbus-5 defaults for oracle peg rebalance (mint + pool swap + leftover burn).
# No LP. Peg: 1 UST1 = $1; cLUNC/cUSTC = LUNC_USD / USTC_USD.
# shellcheck shell=bash

# shellcheck source=ust1-lp-rebalance-defaults.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/ust1-lp-rebalance-defaults.sh"
# shellcheck source=clunc-custc-lp-defaults.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/clunc-custc-lp-defaults.sh"

: "${ORACLE_RB_PAIRS:=both}"
: "${ORACLE_RB_PRICE_TOLERANCE:=0.001}"
: "${ORACLE_RB_SWAP_MAX_SPREAD:=0.20}"
# Pinned gas for the atomic multi-msg swap tx (two CW20 Send+Swap executes).
: "${ORACLE_RB_SWAP_GAS:=3000000}"
: "${ORACLE_RB_MINT_BUFFER_BPS:=50}"
: "${ORACLE_RB_LCD_TIMEOUT:=25}"
: "${ORACLE_RB_INDEXER:=${CLUNC_LP_INDEXER}}"
: "${ORACLE_RB_LCD_URL:=${CLUNC_LP_LCD_URL}}"
