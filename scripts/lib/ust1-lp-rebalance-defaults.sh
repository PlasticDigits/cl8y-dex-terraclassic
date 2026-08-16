#!/usr/bin/env bash
# Columbus-5 defaults for UST1/cUSTC + UST1/USTR LP rebalance / mint-to-CMM.
# Addresses: deployments/mainnet-ust1-wrap/REGISTRY.md + factory Pair queries (2026-08-16).
# shellcheck shell=bash

: "${UST1_LP_FACTORY:=terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea}"
: "${UST1_LP_UST1:=terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72}"
: "${UST1_LP_CUSTC:=terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch}"
: "${UST1_LP_USTR:=terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv}"
: "${UST1_LP_PAIR_CUSTC:=terra1ceprjsxp86ggftf5e38wwt34l83e5gq7penkdnv4wsatkwcs8v6qccw55f}"
: "${UST1_LP_PAIR_USTR:=terra16vxrhpvpcucu05y0nr862vf9hnqeh274uaff4s7hz4n0ea74006qf5hgqy}"
: "${UST1_LP_TREASURY:=terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2}"
: "${UST1_LP_MSIG_ADDR:=terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7}"

# DEX 2-of-3 keyring names (same as scripts/multisig-2of3-host-tx.sh).
: "${UST1_LP_MSIG_KEY:=multisig_2of3}"
: "${UST1_LP_SIGNER1:=multisig1}"
: "${UST1_LP_SIGNER2:=multisig2}"

# Hot wallet that receives minted inventory, swaps, and provides LP.
: "${UST1_LP_ADMIN_KEY:=cl8ydeploy}"
: "${UST1_LP_ADMIN_ADDR:=terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv}"

: "${UST1_LP_USD_EACH:=1000}"
: "${UST1_LP_PRICE_TOLERANCE:=0.001}"
: "${UST1_LP_USTR_PER_USTC:=2.5}"
: "${UST1_LP_INDEXER_ORACLE:=https://indexer.dex.cl8y.com/api/v1/oracle/price/ustc}"
: "${UST1_LP_LCD_URL:=https://terra-classic-lcd.publicnode.com}"
