#!/usr/bin/env bash
# Automated verification for GitLab #435 — proactive anomaly signals (SEC-G02).
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
echo "  GitLab #435 — proactive anomaly signals (SEC-G02)"
echo "════════════════════════════════════════════════════════════════"

run_step "anomaly signals doc invariant" \
  make check-anomaly-signals-docs

run_step "agent skill present" \
  test -f skills/AGENTS_ANOMALY_SIGNALS.md

run_step "incident template Triage links runbook" \
  grep -q 'anomaly-signals.md' docs/templates/incident-dex-indexer.md

run_step "security-posture links anomaly runbook" \
  grep -q 'anomaly-signals.md' docs/security-posture.md

run_step "runbook defines all five signal rows A1–A5" \
  grep -q '\*\*A1\*\*' docs/runbooks/anomaly-signals.md && \
  grep -q '\*\*A5\*\*' docs/runbooks/anomaly-signals.md

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
