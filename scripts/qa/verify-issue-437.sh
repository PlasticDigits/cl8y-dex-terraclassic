#!/usr/bin/env bash
# Automated verification for GitLab #437 — suspicious activity discovery queries (SEC-G04).
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
echo "  GitLab #437 — suspicious activity queries (SEC-G04)"
echo "════════════════════════════════════════════════════════════════"

run_step "suspicious activity doc invariant" \
  make check-suspicious-activity-queries-docs

run_step "agent skill present" \
  test -f skills/AGENTS_SUSPICIOUS_ACTIVITY_QUERIES.md

run_step "incident template Triage links runbook" \
  grep -q 'suspicious-activity-queries.md' docs/templates/incident-dex-indexer.md

run_step "runbook covers wallet discovery (failed tx + volume)" \
  grep -q 'traders/leaderboard' docs/runbooks/suspicious-activity-queries.md && \
  grep -q 'query=message.module' docs/runbooks/suspicious-activity-queries.md && \
  grep -q 'code != 0' docs/runbooks/suspicious-activity-queries.md

run_step "runbook covers pair/token discovery" \
  grep -q 'liquidity_events' docs/runbooks/suspicious-activity-queries.md && \
  grep -q 'compliance/blacklist-check' docs/runbooks/suspicious-activity-queries.md

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
