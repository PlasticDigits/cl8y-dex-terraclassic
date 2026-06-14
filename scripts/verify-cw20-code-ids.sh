#!/usr/bin/env bash
# Verify GDEX / TerraPort (or other) CW20 code IDs via LCD CodeInfo before whitelist.
# GitLab #377 (H-01). Manual operator sign-off still required for fee-on-transfer check.
set -euo pipefail

LCD_URL="${LCD_URL:-https://terra-classic-lcd.publicnode.com}"
EXPECTED_GDEX_CW20_CODE_ID="${EXPECTED_GDEX_CW20_CODE_ID:-}"
EXPECTED_TERRAPORT_CW20_CODE_ID="${EXPECTED_TERRAPORT_CW20_CODE_ID:-}"
EXTRA_CODE_IDS="${EXTRA_CODE_IDS:-}"

query_code() {
  local id="$1"
  local label="$2"
  if [[ -z "$id" ]]; then
    echo "SKIP: $label (env not set)"
    return 0
  fi
  echo "=== $label code_id=$id ==="
  if command -v terrad >/dev/null 2>&1; then
    terrad query wasm code-info "$id" --node "$LCD_URL" -o json
  elif command -v curl >/dev/null 2>&1; then
    curl -fsS "${LCD_URL%/}/cosmwasm/wasm/v1/code/${id}" | python3 -m json.tool
  else
    echo "FAIL: need terrad or curl" >&2
    return 1
  fi
  echo "PASS: CodeInfo retrieved for $label"
}

fail=0
query_code "$EXPECTED_GDEX_CW20_CODE_ID" "GDEX CW20" || fail=1
query_code "$EXPECTED_TERRAPORT_CW20_CODE_ID" "TerraPort CW20" || fail=1

if [[ -n "$EXTRA_CODE_IDS" ]]; then
  IFS=',' read -ra extras <<< "$EXTRA_CODE_IDS"
  for e in "${extras[@]}"; do
    query_code "$e" "extra" || fail=1
  done
fi

if [[ -z "$EXPECTED_GDEX_CW20_CODE_ID" && -z "$EXPECTED_TERRAPORT_CW20_CODE_ID" && -z "$EXTRA_CODE_IDS" ]]; then
  echo "NOTE: Set EXPECTED_GDEX_CW20_CODE_ID / EXPECTED_TERRAPORT_CW20_CODE_ID / EXTRA_CODE_IDS for checks."
  echo "See docs/runbooks/cw20-whitelist-policy.md"
fi

exit "$fail"
