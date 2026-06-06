#!/usr/bin/env bash
# Regression checks for scripts/has-localterra.sh (agent LocalTerra probe).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

_fail() {
  echo "FAIL: $*" >&2
  exit 1
}

grep -q 'localterra_rpc_status_ok' "$REPO_ROOT/scripts/has-localterra.sh" \
  || _fail 'has-localterra.sh must use localterra_rpc_status_ok'
grep -q 'setup-cloud-localterra' "$REPO_ROOT/scripts/has-localterra.sh" \
  || _fail 'has-localterra.sh must mention setup-cloud-localterra'
grep -q 'Cloud Agent VMs support LocalTerra' "$REPO_ROOT/scripts/has-localterra.sh" \
  || _fail 'has-localterra.sh must state Cloud Agent VMs support LocalTerra'

if [[ -x "$REPO_ROOT/scripts/has-localterra.sh" ]]; then
  if "$REPO_ROOT/scripts/has-localterra.sh" --quiet; then
    echo "OK: live LocalTerra probe (has-localterra --quiet exit 0)"
  else
    echo "OK: has-localterra wiring (LocalTerra not up — skipped live exit-0 check)"
  fi
else
  _fail 'scripts/has-localterra.sh must be executable'
fi

echo "OK: has-localterra static checks"
