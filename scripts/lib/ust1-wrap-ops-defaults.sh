#!/usr/bin/env bash
# Shared columbus-5 defaults for UST1 + wrap ops checks (GitLab #503).
# shellcheck shell=bash
# Source only — do not execute.
#
# Canonical addresses: deployments/mainnet-ust1-wrap/REGISTRY.md

: "${UST1_OPS_LCD_URL:=https://terra-classic-lcd.publicnode.com}"
: "${UST1_OPS_CHAIN_ID:=columbus-5}"

# Phase 2
: "${UST1_OPS_WINDOW:=terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2}"
: "${UST1_OPS_ORACLE:=terra1fmht0t6svq3n24zx03nkfja0m40zhfyyxkdcvlrkl6u7gfe6aagq4gch8n}"
: "${UST1_OPS_UST1:=terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72}"
: "${UST1_OPS_VFDUSD:=terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3}"

# Phase 3
: "${UST1_OPS_TREASURY:=terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2}"
: "${UST1_OPS_WRAP_MAPPER:=terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2}"
: "${UST1_OPS_CLUNC:=terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg}"
: "${UST1_OPS_CUSTC:=terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch}"

# Governance (queried on-chain; defaults match 2026-08 mainnet)
: "${UST1_OPS_WRAP_GOVERNANCE:=terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l}"
: "${UST1_OPS_DEX_GOVERNANCE:=terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7}"

# Soft-launch DEX anchors
: "${UST1_OPS_FACTORY:=terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea}"
: "${UST1_OPS_ROUTER:=terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw}"

# Alert / inventory thresholds (override in env for prod tuning)
# Align with ust1-window default + on-chain max_oracle_age_sec (Coolify prod: 21600).
: "${UST1_OPS_ORACLE_SILENCE_SECS:=21600}"       # 6h — ORACLE_MAX_SILENCE_SECS upstream
: "${UST1_OPS_VFDUSD_BALANCE_WARN:=1000000000}"  # 1000 vFDUSD @ 6dp
: "${UST1_OPS_VFDUSD_ALLOWANCE_WARN:=1000000000}"
