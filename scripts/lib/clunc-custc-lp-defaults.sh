#!/usr/bin/env bash
# Columbus-5 defaults for cLUNC/cUSTC v2 LP rebalance / mint-to-CMM.
# Addresses: deployments/mainnet-ust1-wrap/REGISTRY.md + factory Pair (2026-09-04).
# shellcheck shell=bash

: "${CLUNC_LP_FACTORY:=terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea}"
: "${CLUNC_LP_CLUNC:=terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg}"
: "${CLUNC_LP_CUSTC:=terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch}"
: "${CLUNC_LP_PAIR:=terra15rl8g308yzzt5kxu4skgwlahrvm8adyv0s2cupsmvte0akgs2ttsszau38}"
: "${CLUNC_LP_LP_TOKEN:=terra132uuzdnjce0c8g5dalyvdgl47ny697udesk972cg05e5y7gn485qz6tdch}"
: "${CLUNC_LP_TREASURY:=terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2}"
: "${CLUNC_LP_WRAP_MAPPER:=terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2}"

# Primary CW20 minter of cLUNC + cUSTC (wrap-stack EOA). Extra minter on cLUNC is
# wrap-mapper only; DEX 2-of-3 can extra-mint cUSTC but not cLUNC.
: "${CLUNC_LP_MINTER_KEY:=cl8y2_admin}"
: "${CLUNC_LP_MINTER_ADDR:=terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l}"

# Hot wallet that receives minted inventory, swaps, and provides LP.
: "${CLUNC_LP_ADMIN_KEY:=cl8ydeploy}"
: "${CLUNC_LP_ADMIN_ADDR:=terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv}"

# Target TVL rungs (USD, 50/50 each side). Final = $10k ($5k cLUNC + $5k cUSTC).
# mint-clunc-custc-lp.sh sets SKIP_SWAP=1 and ADD_USD=10000 (add $10k at live ratio).
: "${CLUNC_LP_RUNGS:=200,500,2000,5000,10000}"
: "${CLUNC_LP_SKIP_SWAP:=0}"
: "${CLUNC_LP_ADD_USD:=}"
: "${CLUNC_LP_PRICE_TOLERANCE:=0.001}"
: "${CLUNC_LP_CROSS_CHECK_TOLERANCE:=0.03}"
: "${CLUNC_LP_ORACLE_MAX_AGE_SEC:=1800}"
: "${CLUNC_LP_INDEXER:=https://indexer.dex.cl8y.com}"
: "${CLUNC_LP_LCD_URL:=https://terra-classic-lcd.publicnode.com}"

# minter = primary CW20 mint (unbacked wrap supply). wrap = treasury WrapDeposit.
: "${CLUNC_LP_MINT_MODE:=minter}"
