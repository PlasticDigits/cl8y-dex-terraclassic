#!/usr/bin/env bash
# Canonical LocalTerra probe for agents (Cloud Agent VMs: sg docker + exec fallback).
#
# Exit 0 when RPC responds (host curl or docker exec). Exit 1 when down/not ready.
#
# Usage (repo root):
#   ./scripts/has-localterra.sh           # human message + exit code
#   ./scripts/has-localterra.sh --quiet   # exit code only (no stdout on success)
#
# Cloud Agent VMs **can** run LocalTerra via Docker — do not skip chain work without
# running `make setup-cloud-localterra` first. See AGENTS.md § LocalTerra.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

QUIET=0
if [[ "${1:-}" == "--quiet" ]]; then
  QUIET=1
fi

# shellcheck source=scripts/lib/localterra-host-curl.sh
source "$REPO_ROOT/scripts/lib/localterra-host-curl.sh"

TERRA_RPC_URL="${TERRA_RPC_URL:-http://127.0.0.1:${DEX_TERRA_RPC_PORT:-26657}}"

_say() {
  if [[ "$QUIET" -eq 0 ]]; then
    echo "$@"
  fi
}

if localterra_rpc_status_ok "$TERRA_RPC_URL"; then
  cid="$(localterra_container_id "$REPO_ROOT")"
  height=""
  height="$(localterra_host_curl "${TERRA_RPC_URL%/}/status" | jq -r '.result.sync_info.latest_block_height // empty' 2>/dev/null || true)"
  short_cid="${cid:0:12}"
  if [[ -n "$short_cid" ]]; then
    _say "LocalTerra: running (container ${short_cid}, block ${height:-?})"
  else
    _say "LocalTerra: running (block ${height:-?})"
  fi
  exit 0
fi

cid="$(localterra_container_id "$REPO_ROOT")"
if [[ -n "$cid" ]]; then
  _say "LocalTerra: container ${cid:0:12} up but RPC not ready — run: make wait-localterra"
  exit 1
fi

_say "LocalTerra: not running"
_say ""
_say "Cloud Agent VMs support LocalTerra in Docker. Provision (do not skip chain tests without trying):"
_say "  make setup-cloud-localterra"
_say ""
_say "Or step-by-step: make start && make wait-healthy && make build-optimized && make deploy-local"
exit 1
