#!/usr/bin/env bash
# Wait until LocalTerra RPC responds (host curl or docker exec fallback).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/localterra-host-curl.sh
source "$REPO_ROOT/scripts/lib/localterra-host-curl.sh"

RPC_URL="${TERRA_RPC_URL:-http://127.0.0.1:${DEX_TERRA_RPC_PORT:-26657}}"

echo "Waiting for LocalTerra (${RPC_URL})..."
for i in $(seq 1 60); do
  if localterra_rpc_status_ok "$RPC_URL"; then
    echo "LocalTerra is ready!"
    exit 0
  fi
  if [ "$i" -eq 60 ]; then
    echo "ERROR: LocalTerra did not start in time." >&2
    exit 1
  fi
  sleep 2
done
