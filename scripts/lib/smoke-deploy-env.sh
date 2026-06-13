#!/usr/bin/env bash
# Export PAIR_ADDR / OFFER_TOKEN / TERRA_LCD_URL for post-deploy smoke from deploy artifacts.
# shellcheck shell=bash
set -euo pipefail

_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$_LIB_DIR/../.." && pwd)}"
STAMP="${QA_DEPLOY_STAMP_FILE:-$REPO_ROOT/.qa-deploy-stamp}"

set -a
if [ -f "$REPO_ROOT/.env" ]; then
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
fi
if [ -f "$REPO_ROOT/scripts/qa/qa-host.env" ]; then
  # shellcheck source=/dev/null
  source "$REPO_ROOT/scripts/qa/qa-host.env"
fi
set +a

LCD="${TERRA_LCD_URL:-http://127.0.0.1:${DEX_TERRA_LCD_PORT:-1317}}"
LCD="${LCD%/}"
export TERRA_LCD_URL="$LCD"

PAIR="${PAIR_ADDR:-}"
if [ -z "$PAIR" ] && [ -f "$STAMP" ]; then
  # shellcheck disable=SC1090
  source "$STAMP"
  PAIR="${pair_address:-}"
fi

if [ -z "$PAIR" ] && [ -x "$REPO_ROOT/scripts/lib/e2e-trade-pair-from-deploy.sh" ]; then
  PAIR="$("$REPO_ROOT/scripts/lib/e2e-trade-pair-from-deploy.sh" 2>/dev/null || true)"
fi

if [ -z "$PAIR" ]; then
  echo "[smoke-deploy-env] ERROR: could not resolve PAIR_ADDR (set PAIR_ADDR or run deploy-local)." >&2
  exit 1
fi
export PAIR_ADDR="$PAIR"

if [ -z "${OFFER_TOKEN:-}" ]; then
  # shellcheck source=scripts/lib/lcd-smart-query.sh
  source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
  POOL_JSON="$(lcd_smart_query_raw "$LCD" "$PAIR" '{"pool":{}}')"
  OFFER_TOKEN="$(echo "$POOL_JSON" | jq -r '.data.assets[0].info.token.contract_addr // empty')"
fi

if [ -n "${OFFER_TOKEN:-}" ]; then
  export OFFER_TOKEN
fi
