#!/usr/bin/env bash
# Automated verification for GitLab #410 — deploy trace recording (SEC-D12).
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
echo "  GitLab #410 — deploy trace recording (SEC-D12)"
echo "════════════════════════════════════════════════════════════════"

run_step "deploy trace doc invariant" \
  make check-deploy-trace-docs

run_step "deploy trace template present" \
  test -f docs/templates/deploy-trace.md

run_step "agent skill present" \
  test -f skills/AGENTS_DEPLOY_TRACE.md

run_step "launch runbook Phase 1 deploy trace section" \
  grep -q 'Deploy trace (audit record)' docs/runbooks/launch-checklist.md

run_step "wasm migration Pre-flight deploy trace items" \
  grep -q 'Deploy trace (SEC-D12)' docs/runbooks/wasm-admin-migration.md

run_step "deployment guide links deploy trace template" \
  grep -q 'deploy-trace.md' docs/deployment-guide.md

run_step "supply-chain-security cross-links deploy trace" \
  grep -q 'deploy-trace.md' docs/supply-chain-security.md

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
