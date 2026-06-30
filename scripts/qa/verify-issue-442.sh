#!/usr/bin/env bash
# Automated verification for GitLab #442 — env/chain address cross-check (SEC-H04).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }

run_step() {
  local label="$1"
  shift
  echo ""
  echo "[$label]"
  if "$@"; then
    ok "$label"
  else
    bad "$label"
  fi
}

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #442 — env address cross-check (SEC-H04)"
echo "════════════════════════════════════════════════════════════════"

run_step "env address doc invariant" \
  make check-deploy-env-addresses-docs

run_step "verify-env-addresses script present" \
  test -f scripts/qa/verify-env-addresses.sh

run_step "agent skill present" \
  test -f skills/AGENTS_DEPLOY_ENV_ADDRESSES_VERIFY.md

run_step "launch runbook Phase 4 env address step" \
  grep -q 'verify-env-addresses.sh' docs/runbooks/launch-checklist.md

run_step "qa-verify-deploy wires env address check" \
  grep -q 'verify-env-addresses.sh' scripts/qa/verify-deploy.sh

run_step "unit tests (no chain)" \
  make test-qa-verify-env-addresses

if make has-localterra >/dev/null 2>&1; then
  run_step "live LocalTerra env address verification" \
    make qa-verify-env-addresses
else
  echo ""
  echo "[live LocalTerra env address verification]"
  echo "  [SKIP] LocalTerra not running (make setup-cloud-localterra to enable)"
  RESULTS+=("SKIP  live LocalTerra env address verification")
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
