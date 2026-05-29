#!/usr/bin/env bash
# Start Vite for local manual QA (trade/charts). Requires deploy-local env.
# Ensures VITE_FACTORY_ADDRESS comes from frontend-dapp/.env.local (not a stale .env).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENV_LOCAL="$REPO_ROOT/frontend-dapp/.env.local"
ENV_DOT="$REPO_ROOT/frontend-dapp/.env"
STAMP="$REPO_ROOT/.qa-deploy-stamp"

_fail() {
  echo "ERROR: $*" >&2
  exit 1
}

_read_vite_var() {
  local file="$1" key="$2"
  [[ -f "$file" ]] || return 1
  local line
  line="$(grep -E "^${key}=" "$file" 2>/dev/null | tail -n 1 || true)"
  [[ -n "$line" ]] || return 1
  printf '%s' "${line#*=}"
}

_ensure_env_local() {
  if [[ -f "$ENV_LOCAL" ]]; then
    return 0
  fi
  if [[ -f "$STAMP" ]]; then
    echo "[dev-frontend-local] Missing $ENV_LOCAL — run: make deploy-local (or bash scripts/deploy-dex-local.sh)" >&2
  else
    echo "[dev-frontend-local] Missing $ENV_LOCAL — run: make deploy-local from repo root first." >&2
  fi
  _fail "frontend-dapp/.env.local not found"
}

_warn_stale_dot_env() {
  local local_factory dot_factory
  local_factory="$(_read_vite_var "$ENV_LOCAL" VITE_FACTORY_ADDRESS || true)"
  dot_factory="$(_read_vite_var "$ENV_DOT" VITE_FACTORY_ADDRESS || true)"
  if [[ -z "$dot_factory" || -z "$local_factory" ]]; then
    return 0
  fi
  if [[ "$dot_factory" != "$local_factory" ]]; then
    echo "[dev-frontend-local] WARNING: frontend-dapp/.env has VITE_FACTORY_ADDRESS=$dot_factory" >&2
    echo "[dev-frontend-local]          .env.local has VITE_FACTORY_ADDRESS=$local_factory (Vite prefers .env.local)." >&2
    echo "[dev-frontend-local]          Remove stale keys from .env or delete the file to avoid confusion." >&2
  fi
}

_ensure_env_local
_factory="$(_read_vite_var "$ENV_LOCAL" VITE_FACTORY_ADDRESS || true)"
[[ -n "$_factory" ]] || _fail "VITE_FACTORY_ADDRESS is empty in $ENV_LOCAL — re-run make deploy-local"

_warn_stale_dot_env

_lcd="$(_read_vite_var "$ENV_LOCAL" VITE_TERRA_LCD_URL || echo 'http://127.0.0.1:1317')"
_indexer="$(_read_vite_var "$ENV_LOCAL" VITE_INDEXER_URL || echo 'http://127.0.0.1:3001')"
echo "[dev-frontend-local] VITE_FACTORY_ADDRESS=${_factory}"
echo "[dev-frontend-local] VITE_TERRA_LCD_URL=${_lcd}  VITE_INDEXER_URL=${_indexer}"

HOST="${VITE_DEV_HOST:-127.0.0.1}"
PORT="${VITE_DEV_PORT:-5173}"

exec bash "$REPO_ROOT/scripts/with-node.sh" --cwd frontend-dapp -- npm run dev -- --host "$HOST" --port "$PORT" "$@"
