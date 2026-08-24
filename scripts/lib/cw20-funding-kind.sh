#!/usr/bin/env bash
# Shared CW20 funding classifier for LocalTerra QA (GitLab #620).
#
# Gems / TCL8Y stay Mint. Wrap CW20s skip Mint (mapper-only). Community-tax
# tokens (known address or GetLauncherOrigin.launcher set) use Transfer from
# test1. Never fall back to Mint after a Transfer decision (no MintControl on
# the QA tax token).
#
# Source after VITE_* are exported. Does not broadcast txs.

# Echoes: skip | transfer | mint
# Usage: classify_cw20_funding_kind TOKEN [ORIGIN_LAUNCHER]
# ORIGIN_LAUNCHER is GetLauncherOrigin.launcher (empty / "null" = none).
classify_cw20_funding_kind() {
  local token="${1:-}"
  local origin_launcher="${2:-}"
  if [[ -z "$token" ]]; then
    echo mint
    return 0
  fi
  if [[ -n "${VITE_LUNC_C_TOKEN_ADDRESS:-}" && "$token" == "$VITE_LUNC_C_TOKEN_ADDRESS" ]]; then
    echo skip
    return 0
  fi
  if [[ -n "${VITE_USTC_C_TOKEN_ADDRESS:-}" && "$token" == "$VITE_USTC_C_TOKEN_ADDRESS" ]]; then
    echo skip
    return 0
  fi
  if [[ -n "${VITE_TOKEN_COMMUNITY_TAX_ADDRESS:-}" && "$token" == "$VITE_TOKEN_COMMUNITY_TAX_ADDRESS" ]]; then
    echo transfer
    return 0
  fi
  if [[ -n "$origin_launcher" && "$origin_launcher" != "null" ]]; then
    echo transfer
    return 0
  fi
  echo mint
}

# True when TOKEN is the listed QA tax CW20 (env pin only — no LCD).
is_known_community_tax_token() {
  local token="${1:-}"
  [[ -n "${VITE_TOKEN_COMMUNITY_TAX_ADDRESS:-}" && "$token" == "$VITE_TOKEN_COMMUNITY_TAX_ADDRESS" ]]
}
