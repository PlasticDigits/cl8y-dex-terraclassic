#!/usr/bin/env bash
# Automated verification for GitLab #445 — rollback/forward-fix decision tree (SEC-H09).
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
echo "  GitLab #445 — rollback/forward-fix decision tree (SEC-H09)"
echo "════════════════════════════════════════════════════════════════"

run_step "rollback decision doc invariant (SEC-H09 markers)" \
  make check-rollback-decision-docs

run_step "runbook documents all four incident types" \
  grep -q '## 1. Frontend-only incident' docs/runbooks/rollback-decision.md && \
  grep -q '## 2. Indexer incident' docs/runbooks/rollback-decision.md && \
  grep -q '## 3. Contract incident' docs/runbooks/rollback-decision.md && \
  grep -q '## 4. Chain dependency incident' docs/runbooks/rollback-decision.md

run_step "each type includes decision, rollback path, limitations, verification" \
  grep -q '### Decision criteria' docs/runbooks/rollback-decision.md && \
  grep -q '### Rollback path (commands)' docs/runbooks/rollback-decision.md && \
  grep -q '### Limitations' docs/runbooks/rollback-decision.md && \
  grep -q '### Recovery verification' docs/runbooks/rollback-decision.md

run_step "launch-checklist rollback section cross-links runbook" \
  grep -q 'rollback-decision.md' docs/runbooks/launch-checklist.md

run_step "wasm-admin-migration cross-links rollback runbook" \
  grep -q 'rollback-decision.md' docs/runbooks/wasm-admin-migration.md

run_step "emergency-commands cross-links rollback runbook" \
  grep -q 'rollback-decision.md' docs/runbooks/emergency-commands.md

run_step "incident template Mitigation links rollback runbook" \
  grep -q 'rollback-decision.md' docs/templates/incident-dex-indexer.md

run_step "agent skill documents SEC-H09 rollback tree" \
  grep -q 'SEC-H09' skills/AGENTS_ROLLBACK_DECISION.md

run_step "security-model links rollback decision runbook" \
  grep -q 'rollback-decision.md' docs/security-model.md

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
