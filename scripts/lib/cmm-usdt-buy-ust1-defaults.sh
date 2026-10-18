#!/usr/bin/env bash
# Columbus-5 defaults for a CMM USDT → UST1 best-solver drip.
#
# Spend USDT held by the CMM treasury (terra16j5u6…) to buy UST1 through the
# indexer best-execution solver, sending the UST1 back to CMM, until the CMM
# USDT CW20 balance is down to a floor.
#
# Addresses: deployments/mainnet-ust1-wrap/REGISTRY.md
# shellcheck shell=bash

: "${CMM_USDT_BUY_FACTORY:=terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea}"
: "${CMM_USDT_BUY_ROUTER:=terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw}"
: "${CMM_USDT_BUY_UST1:=terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72}"
# Bridged Tether USD CW20 (18 decimals, code 10184). Not the USDT-Bridge-Debt receipt.
: "${CMM_USDT_BUY_USDT:=terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4}"
: "${CMM_USDT_BUY_TREASURY:=terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2}"
: "${CMM_USDT_BUY_MSIG_ADDR:=terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7}"

# DEX 2-of-3 governance (treasury Config.governance + wasm admin).
: "${CMM_USDT_BUY_MSIG_KEY:=multisig_2of3}"
: "${CMM_USDT_BUY_SIGNER1:=multisig1}"
: "${CMM_USDT_BUY_SIGNER2:=multisig2}"

# Hot wallet that pulls USDT from CMM, swaps it, and returns UST1 to CMM.
# cl8ydeploy is deploy-only; use the wrap-stack operations wallet here.
: "${CMM_USDT_BUY_ADMIN_KEY:=cl8y2_admin}"
: "${CMM_USDT_BUY_ADMIN_ADDR:=terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l}"

# Treasury InstantWithdrawCw20 spender for USDT. Must equal the address of
# CMM_USDT_BUY_ADMIN_KEY so the same key can pull and swap. Register it once
# with `--setup-spender` (DEX 2-of-3, no timelock).
: "${CMM_USDT_BUY_SPENDER:=$CMM_USDT_BUY_ADMIN_ADDR}"

# 50 USDT per tick, every 20 minutes, until the CMM USDT floor is reached.
: "${CMM_USDT_BUY_USDT_PER_TICK:=50}"
: "${CMM_USDT_BUY_FLOOR_USDT:=500}"
: "${CMM_USDT_BUY_INTERVAL_SEC:=1200}"

# cmm = pull each tick from the treasury; wallet = only use USDT already on admin.
: "${CMM_USDT_BUY_SOURCE:=cmm}"
: "${CMM_USDT_BUY_DEST:=cmm}"
: "${CMM_USDT_BUY_SLIPPAGE_PERCENT:=0.25}"
: "${CMM_USDT_BUY_MAX_SPREAD:=0.20}"
: "${CMM_USDT_BUY_PULL_LIMIT_24H_USDT:=3600}"

: "${CMM_USDT_BUY_INDEXER:=https://indexer.dex.cl8y.com}"
: "${CMM_USDT_BUY_LCD_URL:=https://terra-classic-lcd.publicnode.com}"
# Bounded retries for the transient indexer 5xx/429 blips (one per tick otherwise).
: "${CMM_USDT_BUY_QUOTE_RETRIES:=4}"
: "${CMM_USDT_BUY_QUOTE_RETRY_DELAY_SEC:=3}"
