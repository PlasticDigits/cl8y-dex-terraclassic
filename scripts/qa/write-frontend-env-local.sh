#!/usr/bin/env bash
# Rewrite VITE_* RPC/LCD/indexer URLs in frontend-dapp/.env.local from scripts/qa/qa-host.env.
# Use after copying .env.local from the QA server if you need to match remapped ports locally.
#
# Usage (repo root):
#   ./scripts/qa/write-frontend-env-local.sh
#   ./scripts/qa/write-frontend-env-local.sh /path/to/copied.env
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SRC="${1:-$REPO_ROOT/frontend-dapp/.env.local}"
OUT="$REPO_ROOT/frontend-dapp/.env.local"

if [ ! -f "$SRC" ]; then
  echo "[write-frontend-env-local] Missing source file: $SRC" >&2
  echo "  Copy from QA server, e.g. scp user@host:.../frontend-dapp/.env.local frontend-dapp/.env.local" >&2
  exit 1
fi

set -a
if [ -f "$REPO_ROOT/.env" ]; then
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
fi
# shellcheck source=/dev/null
source "$REPO_ROOT/scripts/qa/qa-host.env"
set +a

TMP="$(mktemp)"
# Drop old URL lines; preserve the rest (CRLF-safe).
sed 's/\r$//' "$SRC" | grep -v '^VITE_TERRA_RPC_URL=' \
  | grep -v '^VITE_TERRA_LCD_URL=' \
  | grep -v '^VITE_INDEXER_URL=' >"$TMP"

{
  cat "$TMP"
  echo "VITE_TERRA_RPC_URL=http://localhost:${DEX_TERRA_RPC_PORT}"
  echo "VITE_TERRA_LCD_URL=http://localhost:${DEX_TERRA_LCD_PORT}"
  echo "VITE_INDEXER_URL=http://127.0.0.1:${API_PORT}"
} >"$OUT"
rm -f "$TMP"

echo "[write-frontend-env-local] Wrote $OUT (URLs from qa-host.env)."
