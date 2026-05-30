#!/usr/bin/env bash
# Unit checks for localterra host curl + docker exec fallback helpers.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=scripts/lib/localterra-host-curl.sh
source "$REPO_ROOT/scripts/lib/localterra-host-curl.sh"

_fail() {
  echo "FAIL: $*" >&2
  exit 1
}

grep -q 'localterra_rpc_status_ok' "$REPO_ROOT/scripts/qa/verify-deploy.sh" \
  || _fail 'verify-deploy.sh must use localterra_rpc_status_ok'
grep -q 'localterra_host_curl' "$REPO_ROOT/scripts/lib/lcd-smart-query.sh" \
  || _fail 'lcd-smart-query.sh must use localterra_host_curl'
grep -q 'localterra-host-curl' "$REPO_ROOT/scripts/wait-localterra.sh" \
  || _fail 'wait-localterra.sh must source localterra-host-curl'

inner="$(localterra_in_container_url 'http://127.0.0.1:26657/status')"
[ "$inner" = 'http://127.0.0.1:26657/status' ] || _fail "in-container URL map for 26657: got $inner"

cid="$(localterra_container_id "$REPO_ROOT")"
if [ -n "$cid" ]; then
  localterra_rpc_status_ok 'http://127.0.0.1:26657' \
    || _fail 'localterra_rpc_status_ok should pass when compose localterra is up'
  echo "OK: live docker exec RPC probe (container ${cid:0:12})"
else
  echo "OK: helper wiring (no running localterra container — skipped live probe)"
fi

echo "OK: localterra-host-curl helpers"
