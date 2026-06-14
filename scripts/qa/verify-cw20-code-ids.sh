#!/usr/bin/env bash
# Verify CW20 wasm code IDs before factory whitelist (GitLab #377 / H-01).
# Queries LCD CodeInfo and prints a checklist row per ID.
set -euo pipefail

LCD="${LCD:-https://terra-classic-lcd.publicnode.com}"

usage() {
  echo "Usage: $0 [--lcd URL] CODE_ID [CODE_ID ...]" >&2
  echo "Example: $0 --lcd https://terra-classic-lcd.publicnode.com 89 90" >&2
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --lcd)
      LCD="${2:?}"
      shift 2
      ;;
    -h|--help)
      usage
      ;;
    *)
      break
      ;;
  esac
done

if [[ $# -lt 1 ]]; then
  usage
fi

query() {
  local path="$1"
  if command -v terrad >/dev/null 2>&1; then
    terrad query "$path" --node "$LCD" -o json
  else
    curl -fsS "${LCD%/}/cosmos/base/tendermint/v1beta1/node_info" >/dev/null
    curl -fsS "${LCD%/}/cosmwasm/wasm/v1/code/${2}/info" 2>/dev/null || \
      curl -fsS "${LCD%/}/cosmwasm/wasm/v1/code/${2}"
  fi
}

echo "CW20 code ID verification (LCD: $LCD)"
echo "operator_checklist: fee-on-transfer templates MUST NOT be whitelisted (see docs/runbooks/cw20-whitelist-ops.md)"
echo ""

fail=0
for code_id in "$@"; do
  echo "=== code_id=$code_id ==="
  if command -v terrad >/dev/null 2>&1; then
    if ! info=$(terrad query wasm code-info "$code_id" --node "$LCD" -o json 2>&1); then
      echo "FAIL: LCD query failed: $info"
      fail=1
      continue
    fi
    creator=$(echo "$info" | jq -r '.creator // .code_info.creator // empty')
    checksum=$(echo "$info" | jq -r '.data_hash // .code_info.data_hash // empty')
    echo "creator:   $creator"
    echo "data_hash: $checksum"
  else
    if ! info=$(curl -fsS "${LCD%/}/cosmwasm/wasm/v1/code/${code_id}"); then
      echo "FAIL: HTTP query failed for code_id=$code_id"
      fail=1
      continue
    fi
    creator=$(echo "$info" | jq -r '.code_info.creator // .creator // empty')
    checksum=$(echo "$info" | jq -r '.code_info.data_hash // .data_hash // empty')
    echo "creator:   $creator"
    echo "data_hash: $checksum"
  fi
  echo "manual:    confirm checksum vs smartcontracts/artifacts/checksums.txt or release manifest"
  echo "manual:    confirm NOT fee-on-transfer (docs/runbooks/cw20-whitelist-ops.md)"
  echo "PASS:      CodeInfo retrieved (operator must complete manual rows)"
  echo ""
done

if [[ $fail -ne 0 ]]; then
  exit 1
fi

echo "All code IDs returned CodeInfo. Complete manual checklist before AddWhitelistedCodeId."
