#!/usr/bin/env bash
# Columbus-5 defaults for hourly UST1 mint → best-solver cLUNC swap → burn/CMM.
# Addresses: deployments/mainnet-ust1-wrap/REGISTRY.md
# shellcheck shell=bash

: "${UST1_CLUNC_FACTORY:=terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea}"
: "${UST1_CLUNC_ROUTER:=terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw}"
: "${UST1_CLUNC_UST1:=terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72}"
: "${UST1_CLUNC_CLUNC:=terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg}"
: "${UST1_CLUNC_TREASURY:=terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2}"
: "${UST1_CLUNC_MSIG_ADDR:=terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7}"

# DEX 2-of-3 extra minter (same roster as scripts/rebalance-mint-ust1-lp.sh).
: "${UST1_CLUNC_MSIG_KEY:=multisig_2of3}"
: "${UST1_CLUNC_SIGNER1:=multisig1}"
: "${UST1_CLUNC_SIGNER2:=multisig2}"

# Hot wallet that receives minted UST1, executes the solver swap, then burns or sends.
: "${UST1_CLUNC_ADMIN_KEY:=cl8ydeploy}"
: "${UST1_CLUNC_ADMIN_ADDR:=terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv}"

# 50 UST1 / hour until $2500 of cLUNC (LUNC oracle) is burned or sitting on CMM.
: "${UST1_CLUNC_MINT_HUMAN:=50}"
: "${UST1_CLUNC_TARGET_USD:=2500}"
: "${UST1_CLUNC_INTERVAL_SEC:=3600}"
: "${UST1_CLUNC_DEST:=burn}"
: "${UST1_CLUNC_SLIPPAGE_PERCENT:=5}"
: "${UST1_CLUNC_MAX_SPREAD:=0.20}"

: "${UST1_CLUNC_INDEXER:=https://indexer.dex.cl8y.com}"
: "${UST1_CLUNC_INDEXER_ORACLE:=https://indexer.dex.cl8y.com/api/v1/oracle/price/lunc}"
: "${UST1_CLUNC_LCD_URL:=https://terra-classic-lcd.publicnode.com}"
