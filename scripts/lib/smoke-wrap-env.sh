#!/usr/bin/env bash
# Export wrap-mapper / treasury / LUNC-C addresses for post-deploy wrap smoke scripts.
# shellcheck shell=bash
set -euo pipefail

_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$_LIB_DIR/../.." && pwd)}"
FE_ENV="${SMOKE_FE_ENV:-$REPO_ROOT/frontend-dapp/.env.local}"

read_env_var() {
  sed -n "s/^${2}=//p" "$1" 2>/dev/null | head -1
}

if [ ! -f "$FE_ENV" ]; then
  echo "[smoke-wrap-env] ERROR: missing $FE_ENV (run make deploy-local with full seed)." >&2
  exit 1
fi

WRAP_MAPPER_ADDR="$(read_env_var "$FE_ENV" VITE_WRAP_MAPPER_ADDRESS)"
TREASURY_ADDR="$(read_env_var "$FE_ENV" VITE_TREASURY_ADDRESS)"
LUNC_C_ADDR="$(read_env_var "$FE_ENV" VITE_LUNC_C_TOKEN_ADDRESS)"
LCD="$(read_env_var "$FE_ENV" VITE_TERRA_LCD_URL)"

if [ -z "$WRAP_MAPPER_ADDR" ] || [ -z "$TREASURY_ADDR" ] || [ -z "$LUNC_C_ADDR" ]; then
  echo "[smoke-wrap-env] ERROR: wrap addresses missing in $FE_ENV (need VITE_WRAP_MAPPER_ADDRESS, VITE_TREASURY_ADDRESS, VITE_LUNC_C_TOKEN_ADDRESS)." >&2
  exit 1
fi

LCD="${LCD:-http://127.0.0.1:1317}"
LCD="${LCD%/}"

export WRAP_MAPPER_ADDR TREASURY_ADDR LUNC_C_ADDR TERRA_LCD_URL="$LCD"
