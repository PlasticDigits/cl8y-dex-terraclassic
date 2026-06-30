#!/usr/bin/env bash
# Automated verification for GitLab #440 — unpause prerequisite checklist (SEC-G07).
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
echo "  GitLab #440 — unpause prerequisite checklist (SEC-G07)"
echo "════════════════════════════════════════════════════════════════"

run_step "emergency commands doc invariant (SEC-G07 markers)" \
  make check-emergency-commands-docs

run_step "unpause section includes prerequisite checklist" \
  grep -q '### Before you unpause (mandatory)' docs/runbooks/emergency-commands.md

run_step "checklist requires incident reference and approver" \
  grep -q 'Document unpause rationale' docs/runbooks/emergency-commands.md && \
  grep -q 'Log in incident timeline' docs/runbooks/emergency-commands.md

run_step "checklist requires resolution and funds-at-risk confirmation" \
  grep -q 'Confirm triggering condition is resolved' docs/runbooks/emergency-commands.md && \
  grep -q 'Confirm no funds at risk' docs/runbooks/emergency-commands.md

run_step "symmetric cross-link to blacklist rollback checklist" \
  grep -q 'blacklist-decision.md#false-positive-rollback-unblacklist' docs/runbooks/emergency-commands.md && \
  grep -q 'emergency-commands.md#2-unpause-a-pair' docs/runbooks/blacklist-decision.md

run_step "agent skill documents SEC-G07 unpause gate" \
  grep -q 'SEC-G07' skills/AGENTS_EMERGENCY_COMMANDS.md

run_step "security-model links unpause prerequisite" \
  grep -q 'SEC-G07' docs/security-model.md

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
