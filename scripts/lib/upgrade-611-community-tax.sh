#!/usr/bin/env bash
# Shared helpers for scripts/upgrade-611-community-tax.sh (#611 / #612 / #616).
# shellcheck shell=bash

# Factory must never list these (O601-2). Sisters + ALPHA + unused first launcher.
UPGRADE611_NEVER_WHITELIST_FIXED="8654 11612 11613 11614 11620 11621"

upgrade611_die() { echo "ERROR: $*" >&2; exit 1; }

# Return 0 if $1 must not be factory-whitelisted.
upgrade611_is_forbidden_whitelist() {
  local id="$1"
  local extra="${2:-}"
  local x
  for x in $UPGRADE611_NEVER_WHITELIST_FIXED $extra; do
    [[ -n "$x" && "$x" == "$id" ]] && return 0
  done
  return 1
}

upgrade611_assert_whitelist_ok() {
  local id="$1"
  shift
  [[ "$id" =~ ^[0-9]+$ ]] || upgrade611_die "whitelist code_id must be an integer, got: $id"
  if upgrade611_is_forbidden_whitelist "$id" "$*"; then
    upgrade611_die "refusing AddWhitelistedCodeId $id (launcher / AutoLP / ALPHA / unused 11612 / new sister store)"
  fi
  if [[ "$id" == "11611" ]]; then
    upgrade611_die "11611 is already listed. Pass the new token code_id from this store, not 11611."
  fi
}

# Print LCD contracts for a code_id (paginated). One address per line.
upgrade611_lcd_code_contracts() {
  local lcd="$1"
  local code_id="$2"
  lcd="${lcd%/}"
  local key="" raw next
  while true; do
    if [[ -z "$key" ]]; then
      raw="$(localterra_lcd_curl "$lcd" "/cosmwasm/wasm/v1/code/${code_id}/contracts?pagination.limit=100" || true)"
    else
      raw="$(localterra_lcd_curl "$lcd" "/cosmwasm/wasm/v1/code/${code_id}/contracts?pagination.limit=100&pagination.key=${key}" || true)"
    fi
    if [[ -z "$raw" ]]; then
      return 0
    fi
    printf '%s' "$raw" | jq -r '.contracts[]? // empty'
    next="$(printf '%s' "$raw" | jq -r '.pagination.next_key // empty')"
    [[ -n "$next" && "$next" != "null" ]] || break
    key="$next"
  done
}

upgrade611_lcd_code_hash() {
  local lcd="$1"
  local code_id="$2"
  lcd="${lcd%/}"
  local raw
  raw="$(localterra_lcd_curl "$lcd" "/cosmwasm/wasm/v1/code/${code_id}" || true)"
  printf '%s' "$raw" | jq -r '.code_info.data_hash // empty'
}

upgrade611_lcd_contract_info() {
  local lcd="$1"
  local addr="$2"
  lcd="${lcd%/}"
  localterra_lcd_curl "$lcd" "/cosmwasm/wasm/v1/contract/${addr}"
}
