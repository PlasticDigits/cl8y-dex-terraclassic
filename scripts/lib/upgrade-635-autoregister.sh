#!/usr/bin/env bash
# Shared helpers for scripts/upgrade-635-autoregister.sh (#635 leftover of #633).
# shellcheck shell=bash

# Factory must never list these (same gate as #611 / O601-2).
UPGRADE635_NEVER_WHITELIST="8654 11612 11613 11614 11620 11621 11622"

upgrade635_die() { echo "ERROR: $*" >&2; exit 1; }

upgrade635_is_forbidden_whitelist() {
  local id="$1"
  local extra="${2:-}"
  local x
  for x in $UPGRADE635_NEVER_WHITELIST $extra; do
    [[ -n "$x" && "$x" == "$id" ]] && return 0
  done
  return 1
}

upgrade635_assert_whitelist_ok() {
  local id="$1"
  shift
  [[ "$id" =~ ^[0-9]+$ ]] || upgrade635_die "whitelist code_id must be an integer, got: $id"
  if upgrade635_is_forbidden_whitelist "$id" "$*"; then
    upgrade635_die "refusing AddWhitelistedCodeId $id (launcher / AutoLP / ALPHA / unused 11612)"
  fi
  if [[ "$id" == "11611" || "$id" == "11619" || "$id" == "11626" ]]; then
    upgrade635_die "$id is already stored. Pass a new #633 token code_id from this store, not $id."
  fi
}

# Numeric cw2 version from LCD raw contract_info, or empty.
upgrade635_cw2_version() {
  local lcd="$1"
  local addr="$2"
  local key_b64 json data
  lcd="${lcd%/}"
  if [[ "$(uname)" == Darwin ]]; then
    key_b64="$(printf 'contract_info' | base64 | tr -d '\n')"
  else
    key_b64="$(printf 'contract_info' | base64 -w0)"
  fi
  json="$(localterra_lcd_curl "$lcd" "/cosmwasm/wasm/v1/contract/${addr}/raw/${key_b64}" 2>/dev/null || true)"
  data="$(printf '%s' "$json" | jq -r '.data // empty' 2>/dev/null || true)"
  [[ -n "$data" ]] || return 1
  printf '%s' "$data" | base64 -d 2>/dev/null | jq -r '.version // empty'
}
