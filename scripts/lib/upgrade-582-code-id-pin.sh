#!/usr/bin/env bash
# Shared helpers for GitLab #584 / #582 F6 factory-first pair migrate.
# Sourced by scripts/upgrade-582-code-id-pin.sh and qa tests.
# shellcheck shell=bash

UPGRADE582_PAIR_PAGE_LIMIT="${UPGRADE582_PAIR_PAGE_LIMIT:-30}"
UPGRADE582_MIN_FACTORY_VERSION="${UPGRADE582_MIN_FACTORY_VERSION:-1.9.0}"
UPGRADE582_PAIR_VERSION="${UPGRADE582_PAIR_VERSION:-1.15.0}"
# Probe sample code id when the factory has no listed assets yet (LocalTerra empty).
UPGRADE582_WHITELIST_PROBE_CODE_ID="${UPGRADE582_WHITELIST_PROBE_CODE_ID:-10184}"
UPGRADE582_LCD_RETRIES="${UPGRADE582_LCD_RETRIES:-5}"
UPGRADE582_LCD_RETRY_DELAY_SEC="${UPGRADE582_LCD_RETRY_DELAY_SEC:-2}"
# Safety cap for RefreshPairAssetCodeIdsBatch has_more loops (30 pairs/page).
UPGRADE582_REFRESH_MAX_BATCHES="${UPGRADE582_REFRESH_MAX_BATCHES:-64}"

upgrade582_die() {
  echo "ERROR: $*" >&2
  exit 1
}

# True when $1 >= $2 using sort -V (major.minor.patch).
upgrade582_version_ge() {
  local have="$1"
  local need="$2"
  [[ -n "$have" && -n "$need" ]] || return 1
  local first
  first="$(printf '%s\n%s\n' "$need" "$have" | sort -V | head -1)"
  [[ "$first" == "$need" ]]
}

upgrade582_lcd_base() {
  printf '%s' "${LCD_URL%/}"
}

# GET LCD path (leading slash optional). Uses LocalTerra host/exec fallback.
upgrade582_lcd_get() {
  local path="$1"
  local lcd
  lcd="$(upgrade582_lcd_base)"
  [[ -n "$lcd" ]] || upgrade582_die "LCD_URL is empty"
  localterra_lcd_curl "$lcd" "$path"
}

# LCD GET /cosmwasm/wasm/v1/contract/{addr} JSON (empty on HTTP failure).
upgrade582_contract_info_json() {
  local addr="$1"
  [[ -n "$addr" ]] || return 1
  upgrade582_lcd_get "/cosmwasm/wasm/v1/contract/${addr}"
}

# Numeric ContractInfo.code_id or empty.
upgrade582_contract_info_code_id() {
  local addr="$1"
  local json
  json="$(upgrade582_contract_info_json "$addr" 2>/dev/null || true)"
  printf '%s' "$json" | jq -r '.contract_info.code_id // empty' 2>/dev/null || true
}

# Fail closed unless LCD returns a numeric code_id.
upgrade582_require_contract_info_code_id() {
  local addr="$1"
  local label="${2:-$addr}"
  local json code
  json="$(upgrade582_contract_info_json "$addr" 2>/dev/null || true)"
  [[ -n "$json" ]] || upgrade582_die "ContractInfo probe failed for $label ($addr) on $(upgrade582_lcd_base)/cosmwasm/wasm/v1/contract/{addr}"
  code="$(printf '%s' "$json" | jq -r '.contract_info.code_id // empty')"
  [[ "$code" =~ ^[0-9]+$ ]] || upgrade582_die "ContractInfo.code_id missing/non-numeric for $label ($addr): $(printf '%s' "$json" | jq -c . 2>/dev/null || echo "$json")"
  printf '%s' "$code"
}

upgrade582_query_smart() {
  local contract="$1"
  local msg="$2"
  local attempts="${UPGRADE582_LCD_RETRIES:-5}"
  local delay="${UPGRADE582_LCD_RETRY_DELAY_SEC:-2}"
  local i raw decoded
  for i in $(seq 1 "$attempts"); do
    raw="$(lcd_smart_query_raw "$(upgrade582_lcd_base)" "$contract" "$msg" 2>/dev/null || true)"
    decoded="$(lcd_decode_smart_data "$raw" 2>/dev/null || true)"
    if [[ -n "$decoded" && "$decoded" != "null" ]]; then
      printf '%s' "$decoded"
      return 0
    fi
    echo "  LCD smart-query retry $i/$attempts $contract" >&2
    sleep "$delay"
    delay=$((delay + 1))
  done
  return 1
}

# cw2 version from LCD raw `contract_info` key, or UPGRADE582_FORCE_FACTORY_VERSION for tests.
upgrade582_cw2_version() {
  local addr="$1"
  if [[ -n "${UPGRADE582_FORCE_FACTORY_VERSION:-}" && "$addr" == "${FACTORY:-}" ]]; then
    printf '%s' "$UPGRADE582_FORCE_FACTORY_VERSION"
    return 0
  fi
  if [[ -n "${UPGRADE582_FORCE_CW2_VERSION:-}" ]]; then
    printf '%s' "$UPGRADE582_FORCE_CW2_VERSION"
    return 0
  fi
  local key_b64 json data ver
  if [[ "$(uname)" == Darwin ]]; then
    key_b64="$(printf 'contract_info' | base64 | tr -d '\n')"
  else
    key_b64="$(printf 'contract_info' | base64 -w0)"
  fi
  json="$(upgrade582_lcd_get "/cosmwasm/wasm/v1/contract/${addr}/raw/${key_b64}" 2>/dev/null || true)"
  data="$(printf '%s' "$json" | jq -r '.data // empty' 2>/dev/null || true)"
  if [[ -n "$data" ]]; then
    ver="$(printf '%s' "$data" | base64 -d 2>/dev/null | jq -r '.version // empty' 2>/dev/null || true)"
    if [[ -n "$ver" ]]; then
      printf '%s' "$ver"
      return 0
    fi
  fi
  return 1
}

# Overridable: fetch one factory Pairs page. Prints JSON {pairs:[...]}.
# start_after_json is empty (first page) or a JSON array of two AssetInfos.
upgrade582_fetch_pairs_page() {
  local start_after_json="${1:-}"
  local limit="${2:-$UPGRADE582_PAIR_PAGE_LIMIT}"
  local msg
  if [[ -z "$start_after_json" || "$start_after_json" == "null" ]]; then
    msg="$(jq -nc --argjson limit "$limit" '{pairs:{start_after:null,limit:$limit}}')"
  else
    msg="$(jq -nc --argjson sa "$start_after_json" --argjson limit "$limit" \
      '{pairs:{start_after:$sa,limit:$limit}}')"
  fi
  upgrade582_query_smart "$FACTORY" "$msg"
}

# Enumerate all factory pairs at limit 30 with start_after = last asset_infos.
# Prints one JSON object per pair (contract_addr + asset_infos).
# Sets UPGRADE582_PAIRS_PAGES to the number of LCD queries issued.
upgrade582_enumerate_pairs() {
  local start_after=""
  local page n
  UPGRADE582_PAIRS_PAGES=0
  while true; do
    page="$(upgrade582_fetch_pairs_page "$start_after" "$UPGRADE582_PAIR_PAGE_LIMIT")"
    UPGRADE582_PAIRS_PAGES=$((UPGRADE582_PAIRS_PAGES + 1))
    n="$(printf '%s' "$page" | jq -r '.pairs | length // 0')"
    [[ "$n" =~ ^[0-9]+$ ]] || upgrade582_die "pairs page did not parse (need .pairs[])"
    if [[ "$n" -eq 0 ]]; then
      break
    fi
    printf '%s' "$page" | jq -c '.pairs[]'
    if [[ "$n" -lt "$UPGRADE582_PAIR_PAGE_LIMIT" ]]; then
      break
    fi
    start_after="$(printf '%s' "$page" | jq -c '.pairs[-1].asset_infos')"
    [[ -n "$start_after" && "$start_after" != "null" ]] \
      || upgrade582_die "pairs page was full but last asset_infos missing (cannot paginate)"
  done
}

upgrade582_get_pair_count() {
  upgrade582_query_smart "$FACTORY" '{"get_pair_count":{}}' | jq -r '.count // empty'
}

upgrade582_is_code_id_whitelisted_json() {
  local code_id="$1"
  local msg
  if [[ -n "${UPGRADE582_FORCE_WHITELIST_JSON:-}" ]]; then
    printf '%s' "$UPGRADE582_FORCE_WHITELIST_JSON"
    return 0
  fi
  # DRY_RUN without live LCD still exercises the jq boolean parse (issue notes:
  # skipping this query when cw2 ≥1.9.0 left the gate untested).
  if [[ "${DRY_RUN:-0}" == "1" && -z "${UPGRADE582_DRY_QUERY:-}" ]]; then
    printf '%s' '{"whitelisted":true}'
    return 0
  fi
  msg="$(jq -nc --argjson id "$code_id" '{is_code_id_whitelisted:{code_id:$id}}')"
  upgrade582_query_smart "$FACTORY" "$msg"
}

# Boolean `true`/`false` from IsCodeIdWhitelisted. Retries LCD flakes that
# drop `.whitelisted` (columbus-5 smoke aborted as pin1=10184/null — the pin
# was fine; the second whitelist read returned empty). FORCE_WHITELIST_JSON
# is single-shot so verify can refuse without sleeping.
upgrade582_whitelist_bool() {
  local code_id="$1"
  local attempts="${UPGRADE582_LCD_RETRIES:-5}"
  local delay="${UPGRADE582_LCD_RETRY_DELAY_SEC:-2}"
  local i json val
  if [[ -n "${UPGRADE582_FORCE_WHITELIST_JSON:-}" ]]; then
    attempts=1
  fi
  for i in $(seq 1 "$attempts"); do
    json="$(upgrade582_is_code_id_whitelisted_json "$code_id" 2>/dev/null || true)"
    val="$(printf '%s' "$json" | jq -r 'if .whitelisted==true or .whitelisted==false then (.whitelisted|tostring) else empty end' 2>/dev/null || true)"
    if [[ "$val" == "true" || "$val" == "false" ]]; then
      printf '%s' "$val"
      return 0
    fi
    echo "  IsCodeIdWhitelisted retry $i/$attempts code_id=$code_id (not a boolean; LCD flake ≠ empty pin)" >&2
    [[ "$i" -lt "$attempts" ]] && sleep "$delay"
  done
  return 1
}

upgrade582_factory_config_json() {
  if [[ -n "${UPGRADE582_FORCE_CONFIG_JSON:-}" ]]; then
    printf '%s' "$UPGRADE582_FORCE_CONFIG_JSON"
    return 0
  fi
  if [[ "${DRY_RUN:-0}" == "1" && -z "${UPGRADE582_DRY_QUERY:-}" ]]; then
    printf '%s' '{"pair_code_id":0}'
    return 0
  fi
  upgrade582_query_smart "$FACTORY" '{"config":{}}'
}

upgrade582_factory_pair_code_id() {
  local json
  json="$(upgrade582_factory_config_json)"
  printf '%s' "$json" | jq -r '.pair_code_id // empty'
}

# Last wasm attribute value from terrad query-tx JSON (SDK 0.53 .events or legacy .logs).
upgrade582_tx_wasm_attr() {
  local tx_json="$1"
  local key="$2"
  printf '%s' "$tx_json" | jq -r --arg k "$key" \
    '[(.events // .logs[]?.events // [])[] | select((.type // "") | test("wasm")) | .attributes[]? | select(.key == $k) | .value] | last // empty'
}

# Prints "<has_more>\t<next_start_after-or-empty>". Fails unless has_more is true/false.
upgrade582_refresh_batch_cursor() {
  local tx_json="$1"
  local has next
  has="$(upgrade582_tx_wasm_attr "$tx_json" has_more)"
  next="$(upgrade582_tx_wasm_attr "$tx_json" next_start_after)"
  [[ "$has" == "true" || "$has" == "false" ]] || return 1
  printf '%s\t%s\n' "$has" "$next"
}

# Unique CW20 addrs from pair asset_infos (token.contract_addr only).
upgrade582_unique_listed_assets() {
  local pairs_json_lines="$1"
  printf '%s\n' "$pairs_json_lines" | jq -r '
    .asset_infos[]?
    | if type=="object" then
        (.token.contract_addr // .native_token.denom // empty)
      else empty end
  ' | awk 'NF && !seen[$0]++'
}

upgrade582_print_batch_refresh_skip() {
  cat <<'EOF' >&2
Batch RefreshPairAssetCodeIdsBatch is all-or-nothing: one unrefreshable pair
(unlisted live code_id) reverts the whole tx, blocking later-indexed pairs.

Skip procedure (copy-paste; start_after is the PAIR_INDEX of the bad pair):

  # 1) Refresh known-good neighbors one at a time
  terrad tx wasm execute "$FACTORY" \
    "$(jq -nc --arg p "$GOOD_PAIR" '{refresh_pair_asset_code_ids:{pair:$p}}')" \
    --from "$GOVERNANCE_KEY" --chain-id "$CHAIN_ID" --node "$NODE" --gas auto -y

  # 2) Batch from start_after past the bad index (exclusive). Example: bad index=4
  #    (0-based PAIR_INDEX). start_after=4 starts at index 5. has_more on the
  #    wasm event: rerun until has_more=false.
  terrad tx wasm execute "$FACTORY" \
    '{"refresh_pair_asset_code_ids_batch":{"start_after":4,"limit":30}}' \
    --from "$GOVERNANCE_KEY" --chain-id "$CHAIN_ID" --node "$NODE" --gas auto -y

  # Incident pair stays frozen until policy: BlacklistPair / BlacklistToken,
  # or source-review + AddWhitelistedCodeId then single RefreshPairAssetCodeIds.
  # Do not Refresh onto FoT/rebase. Do not RemoveWhitelistedCodeId(10184).
EOF
}
