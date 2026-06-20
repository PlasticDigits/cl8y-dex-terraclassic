#!/usr/bin/env bash
# Automated verification for GitLab #407 — IBC-hooks deploy runbook gate (SEC-D02).
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
echo "  GitLab #407 — IBC-hooks deploy runbook (SEC-D02)"
echo "════════════════════════════════════════════════════════════════"

run_step "IBC-hooks deploy doc invariant" \
  make check-ibc-hooks-deploy-docs

run_step "agent skill present" \
  test -f skills/AGENTS_IBC_HOOKS_DEPLOY.md

run_step "static contract IBC entry-point grep" \
  make verify-no-ibc-hooks-in-contracts

run_step "launch runbook Phase 0 SEC-D02 item" \
  grep -q 'SEC-D02' docs/runbooks/launch-checklist.md

run_step "deployment-guide post-deploy SEC-D02 item" \
  grep -q 'SEC-D02' docs/deployment-guide.md

run_step "security-model IBC hooks section" \
  grep -q 'IBC hooks chain dependency (SEC-D02)' docs/security-model.md

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
