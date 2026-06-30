#!/usr/bin/env bash
# Automated verification for GitLab #444 — pre-deploy test evidence gate (SEC-H08).
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
echo "  GitLab #444 — pre-deploy test evidence gate (SEC-H08)"
echo "════════════════════════════════════════════════════════════════"

run_step "test evidence gate doc invariant" \
  make check-test-evidence-gate-docs

run_step "agent skill present" \
  test -f skills/AGENTS_TEST_EVIDENCE_GATE.md

run_step "launch runbook Phase 0 SEC-H08 item" \
  grep -q 'SEC-H08' docs/runbooks/launch-checklist.md

run_step "launch runbook requires test-contracts output" \
  grep -q 'make test-contracts' docs/runbooks/launch-checklist.md

run_step "launch runbook requires test-indexer-integration output" \
  grep -q 'make test-indexer-integration' docs/runbooks/launch-checklist.md

run_step "launch runbook requires test-frontend output" \
  grep -q 'make test-frontend' docs/runbooks/launch-checklist.md

run_step "launch runbook notes CI-built artifacts" \
  grep -q 'CI-built artifacts' docs/runbooks/launch-checklist.md

run_step "deploy trace template Test results section" \
  grep -q 'Test results (pre-deploy evidence — SEC-H08)' docs/templates/deploy-trace.md

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
