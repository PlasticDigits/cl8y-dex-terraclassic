#!/usr/bin/env bash
# Columbus-5 defaults for hourly $200 cUSTC mint → best-solver UST1 swap until
# CMM vFDUSD CW20 × Venus × FDUSD oracle is $500.
# Addresses: deployments/mainnet-ust1-wrap/REGISTRY.md
# shellcheck shell=bash

: "${CUSTC_UST1_FACTORY:=terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea}"
: "${CUSTC_UST1_ROUTER:=terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw}"
: "${CUSTC_UST1_UST1:=terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72}"
: "${CUSTC_UST1_CUSTC:=terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch}"
: "${CUSTC_UST1_VFDUSD:=terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3}"
: "${CUSTC_UST1_TREASURY:=terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2}"
: "${CUSTC_UST1_MSIG_ADDR:=terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7}"

# DEX 2-of-3 extra minter (cUSTC extras include this roster; same as UST1 LP mint).
: "${CUSTC_UST1_MSIG_KEY:=multisig_2of3}"
: "${CUSTC_UST1_SIGNER1:=multisig1}"
: "${CUSTC_UST1_SIGNER2:=multisig2}"

# Hot wallet that receives minted cUSTC, executes the solver swap, then dest.
: "${CUSTC_UST1_ADMIN_KEY:=cl8ydeploy}"
: "${CUSTC_UST1_ADMIN_ADDR:=terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv}"

# $200 of cUSTC / hour (USTC oracle); stop when CMM vFDUSD USD >= 500.
: "${CUSTC_UST1_MINT_USD:=200}"
: "${CUSTC_UST1_TARGET_USD:=500}"
: "${CUSTC_UST1_INTERVAL_SEC:=3600}"
: "${CUSTC_UST1_DEST:=cmm}"
: "${CUSTC_UST1_SLIPPAGE_PERCENT:=5}"
: "${CUSTC_UST1_MAX_SPREAD:=0.20}"

: "${CUSTC_UST1_INDEXER:=https://indexer.dex.cl8y.com}"
: "${CUSTC_UST1_INDEXER_USTC:=https://indexer.dex.cl8y.com/api/v1/oracle/price/ustc}"
: "${CUSTC_UST1_INDEXER_VFDUSD:=https://indexer.dex.cl8y.com/api/v1/oracle/price/vfdusd}"
: "${CUSTC_UST1_LCD_URL:=https://terra-classic-lcd.publicnode.com}"
