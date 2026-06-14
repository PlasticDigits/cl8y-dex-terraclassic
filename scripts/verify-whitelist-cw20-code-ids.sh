#!/usr/bin/env bash
# Verify factory-whitelisted CW20 code IDs are standard (non fee-on-transfer) templates.
# GitLab #377 (H-01): ops gate before AddWhitelistedCodeId / launch.
set -euo pipefail

LCD_URL="${LCD_URL:-https://terra-classic-lcd.publicnode.com}"
EXPECTED_CW20_CODE_IDS="${EXPECTED_CW20_CODE_IDS:-}"
EXPECTED_CW20_CHECKSUMS="${EXPECTED_CW20_CHECKSUMS:-}"

if [[ -z "$EXPECTED_CW20_CODE_IDS" ]]; then
  echo "ERROR: set EXPECTED_CW20_CODE_IDS (comma-separated code IDs to verify)" >&2
  exit 1
fi

IFS=',' read -r -a CODE_IDS <<< "$EXPECTED_CW20_CODE_IDS"
IFS=',' read -r -a CHECKSUMS <<< "$EXPECTED_CW20_CHECKSUMS"

if [[ -n "$EXPECTED_CW20_CHECKSUMS" && ${#CODE_IDS[@]} -ne ${#CHECKSUMS[@]} ]]; then
  echo "ERROR: EXPECTED_CW20_CODE_IDS and EXPECTED_CW20_CHECKSUMS length mismatch" >&2
  exit 1
fi

query_code_info() {
  local code_id="$1"
  curl -fsS "${LCD_URL%/}/cosmwasm/wasm/v1/code/${code_id}" \
    | jq -r '.code_info | "\(.code_id)|\(.checksum)|\(.creator)"'
}

echo "LCD: $LCD_URL"
echo "Verifying ${#CODE_IDS[@]} CW20 code ID(s)..."

i=0
for code_id in "${CODE_IDS[@]}"; do
  code_id="$(echo "$code_id" | tr -d ' ')"
  [[ -z "$code_id" ]] && continue
  line="$(query_code_info "$code_id")"
  got_id="${line%%|*}"
  rest="${line#*|}"
  got_checksum="${rest%%|*}"
  got_creator="${rest#*|}"

  echo "  code_id=$got_id checksum=$got_checksum creator=$got_creator"

  if [[ -n "$EXPECTED_CW20_CHECKSUMS" ]]; then
    want="${CHECKSUMS[$i]}"
    want="$(echo "$want" | tr -d ' ')"
    if [[ "$got_checksum" != "$want" ]]; then
      echo "ERROR: code_id $code_id checksum mismatch (got $got_checksum, want $want)" >&2
      exit 1
    fi
  fi
  i=$((i + 1))
done

echo "OK: all code IDs resolved on LCD; checksums match when EXPECTED_CW20_CHECKSUMS set."
echo "Reminder: fee-on-transfer templates must never be whitelisted — see docs/runbooks/cw20-code-id-ops.md"
