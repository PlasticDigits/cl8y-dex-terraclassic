#!/usr/bin/env bash
# Automated verification for GitLab #429 — extension fee guard scope (SEC-E08).
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
echo "  GitLab #429 — extension fee guard scope (SEC-E08)"
echo "════════════════════════════════════════════════════════════════"

run_step "extension fee guard doc invariant" \
  make check-extension-fee-guard-docs

run_step "agent skill present" \
  test -f skills/AGENTS_EXTENSION_FEE_GUARD.md

run_step "wallet QA runbook present" \
  test -f docs/runbooks/extension-fee-guard-wallet-qa.md

run_step "launch runbook Phase 4 SEC-E08 item" \
  grep -q 'SEC-E08' docs/runbooks/launch-checklist.md

run_step "security-model SEC-E08 section" \
  grep -q 'Extension wallet fee guard (SEC-E08)' docs/security-model.md

run_step "extensionSignedFeeGuard unit tests (mainnet skip)" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/utils/__tests__/extensionSignedFeeGuard.test.ts

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
