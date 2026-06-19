#!/usr/bin/env bash
# Operator helper: print chain version + IBC-hooks exposure for deploy records (SEC-D02 / #407).
# Usage: TERRA_LCD_URL=https://lcd.terra.dev ./scripts/lib/record-chain-ibc-hooks-version.sh
set -euo pipefail

LCD_URL="${TERRA_LCD_URL:-${TERRA_LCD:-http://127.0.0.1:1317}}"
NODE_FLAG=(--node "$LCD_URL")

if ! command -v terrad >/dev/null 2>&1; then
  echo "ERROR: terrad not found on PATH" >&2
  exit 1
fi

echo "== SEC-D02 chain + IBC-hooks record =="
echo "lcd_url=$LCD_URL"
echo "recorded_at_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""
echo "-- terrad version --long --"
terrad version --long "${NODE_FLAG[@]}" 2>/dev/null || terrad version --long
echo ""
echo "-- params subspaces (ibchooks) --"
if terrad query params subspaces "${NODE_FLAG[@]}" 2>/dev/null | grep -i ibchooks; then
  :
else
  echo "IBC-hooks params subspace: not listed on this node"
fi
echo ""
echo "-- upgrade module_versions (ibchooks, if supported) --"
if terrad query upgrade module_versions "${NODE_FLAG[@]}" 2>/dev/null | grep -i ibchooks; then
  :
else
  echo "IBC-hooks module_versions: not reported (query unsupported or module absent)"
fi
