#!/usr/bin/env bash
# Automated verification for GitLab #441 — post-deploy config assertions (SEC-H03).
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
echo "  GitLab #441 — post-deploy config verification (SEC-H03)"
echo "════════════════════════════════════════════════════════════════"

run_step "deploy config doc invariant" \
  make check-deploy-config-docs

run_step "verify-deploy-config script present" \
  test -f scripts/qa/verify-deploy-config.sh

run_step "agent skill present" \
  test -f skills/AGENTS_DEPLOY_CONFIG_VERIFY.md

run_step "launch runbook Phase 3 config verification step" \
  grep -q 'verify-deploy-config.sh' docs/runbooks/launch-checklist.md

run_step "deploy trace template references config script" \
  grep -q 'verify-deploy-config.sh' docs/templates/deploy-trace.md

run_step "unit tests (no chain)" \
  make test-qa-verify-deploy-config

if make has-localterra >/dev/null 2>&1; then
  run_step "live LocalTerra config verification" \
    make qa-verify-deploy-config
else
  echo ""
  echo "[live LocalTerra config verification]"
  echo "  [SKIP] LocalTerra not running (make setup-cloud-localterra to enable)"
  RESULTS+=("SKIP  live LocalTerra config verification")
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
