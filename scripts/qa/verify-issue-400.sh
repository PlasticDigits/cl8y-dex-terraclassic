#!/usr/bin/env bash
# Automated verification for GitLab #400 — blacklist decision runbook (SEC-B12).
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
echo "  GitLab #400 — blacklist decision runbook (SEC-B12)"
echo "════════════════════════════════════════════════════════════════"

run_step "blacklist decision doc invariant" \
  make check-blacklist-decision-docs

run_step "agent skill present" \
  test -f skills/AGENTS_BLACKLIST_DECISION.md

run_step "incident template Mitigation links runbook" \
  grep -q 'blacklist-decision.md' docs/templates/incident-dex-indexer.md

run_step "security-model links operator runbook" \
  grep -q 'blacklist-decision.md' docs/security-model.md

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
