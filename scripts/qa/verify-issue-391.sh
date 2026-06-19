#!/usr/bin/env bash
# Automated verification for GitLab #391 — launch go/no-go gate in runbook (SEC-A06).
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
echo "  GitLab #391 — launch go/no-go gate (SEC-A06)"
echo "════════════════════════════════════════════════════════════════"

run_step "launch runbook go/no-go doc invariant" \
  make check-launch-go-no-go-docs

run_step "agent skill present" \
  test -f skills/AGENTS_LAUNCH_GO_NO_GO.md

run_step "QA_TEMPLATE SIGN-OFF section present" \
  grep -q '^## SIGN-OFF' QA_TEMPLATE.md

run_step "deployment-guide links launch Phase 5 gate" \
  grep -q 'Phase 5' docs/deployment-guide.md

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
