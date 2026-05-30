#!/usr/bin/env bash
# Shared LCD cosmwasm smart-query helpers for shell scripts (QA verify, E2E seed, smoke).
# shellcheck shell=bash

_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/localterra-host-curl.sh
source "$_LIB_DIR/localterra-host-curl.sh"

lcd_b64_query() {
  local msg="$1"
  if [[ "$(uname)" == Darwin ]]; then
    printf '%s' "$msg" | base64 | tr -d '\n'
  else
    printf '%s' "$msg" | base64 -w0
  fi
}

# GET smart query; prints raw JSON body. Returns non-zero on curl failure.
lcd_smart_query_raw() {
  local lcd="$1"
  local contract="$2"
  local msg_json="$3"
  lcd="${lcd%/}"
  local b64
  b64="$(lcd_b64_query "$msg_json")"
  localterra_lcd_curl "$lcd" "/cosmwasm/wasm/v1/contract/${contract}/smart/${b64}"
}

# GET smart query; prints HTTP status to stdout on failure path via lcd_smart_query_checked.
lcd_smart_query_http() {
  local lcd="$1"
  local contract="$2"
  local msg_json="$3"
  lcd="${lcd%/}"
  local b64
  b64="$(lcd_b64_query "$msg_json")"
  local url="${lcd}/cosmwasm/wasm/v1/contract/${contract}/smart/${b64}"
  local body code
  body="$(localterra_host_curl "$url" 2>/dev/null || true)"
  if [ -n "$body" ]; then
    printf '%s\n200' "$body"
    return 0
  fi
  curl -sS -w $'\n%{http_code}' \
    --connect-timeout "${LOCALTERRA_CURL_CONNECT_TIMEOUT:-2}" \
    --max-time "${LOCALTERRA_CURL_MAX_TIME:-8}" \
    "$url"
}

# Decode `.data` from LCD smart-query JSON (base64 string or inline object).
lcd_decode_smart_data() {
  local raw="$1"
  local data_type
  data_type="$(echo "$raw" | jq -r '.data | type')"
  if [[ "$data_type" == "string" ]]; then
    echo "$raw" | jq -r '.data | @base64d | fromjson'
  elif [[ "$data_type" == "object" || "$data_type" == "number" || "$data_type" == "boolean" ]]; then
    echo "$raw" | jq '.data'
  else
    echo "null"
  fi
}

# Returns 0 when LCD responds HTTP 2xx with parseable smart-query payload (no RPC error envelope).
lcd_smart_query_ok() {
  local lcd="$1"
  local contract="$2"
  local msg_json="$3"
  local combined body code
  combined="$(lcd_smart_query_http "$lcd" "$contract" "$msg_json")"
  code="$(echo "$combined" | tail -n1)"
  body="$(echo "$combined" | sed '$d')"
  if [[ ! "$code" =~ ^2 ]]; then
    return 1
  fi
  if echo "$body" | jq -e '.code != null and .code != 0' >/dev/null 2>&1; then
    return 1
  fi
  if echo "$body" | jq -e '.message != null and (.message | test("unknown variant|Error parsing|execute wasm contract failed"; "i"))' >/dev/null 2>&1; then
    return 1
  fi
  return 0
}
