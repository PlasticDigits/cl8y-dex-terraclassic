#!/usr/bin/env bash
# Columbus-5 defaults for a new cLUNC/USDT v2 pair: mint $1k cLUNC from the
# LUNC/USD oracle, seed with $1k USDT already on cl8y2_admin.
# shellcheck shell=bash

: "${CLUNC_USDT_FACTORY:=terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea}"
: "${CLUNC_USDT_CLUNC:=terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg}"
: "${CLUNC_USDT_USDT:=terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4}"
: "${CLUNC_USDT_TREASURY:=terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2}"

# One wallet: primary cLUNC minter + USDT holder. Do not mint USDT (2-of-3 minter).
: "${CLUNC_USDT_ADMIN_KEY:=cl8y2_admin}"
: "${CLUNC_USDT_ADMIN_ADDR:=terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l}"

# Seed is $USD_EACH of cLUNC (oracle) + $USD_EACH of USDT (assume $1).
: "${CLUNC_USDT_USD_EACH:=1000}"
: "${CLUNC_USDT_USDT_USD:=1}"
: "${CLUNC_USDT_ORACLE_MAX_AGE_SEC:=1800}"
: "${CLUNC_USDT_PRICE_MOVE_TOLERANCE:=0.01}"
: "${CLUNC_USDT_INDEXER:=https://indexer.dex.cl8y.com}"
: "${CLUNC_USDT_INDEXER_ORACLE:=https://indexer.dex.cl8y.com/api/v1/oracle/price/lunc}"
: "${CLUNC_USDT_LCD_URL:=https://terra-classic-lcd.publicnode.com}"
: "${CLUNC_USDT_EXPECTED_CW20_CODE_ID:=10184}"
: "${CLUNC_USDT_GAS_RESERVE_ULUNA:=2000000000}"

# LP receiver for provide_liquidity. CMM is the economic-LP sink used by the
# cLUNC/cUSTC mint scripts. Set to the admin address to keep LP on the ops wallet.
: "${CLUNC_USDT_LP_RECEIVER:=terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2}"

: "${CLUNC_USDT_DEPLOY_DIR_REL:=deployments/clunc-usdt-pair}"
