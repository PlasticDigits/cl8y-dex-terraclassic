#!/usr/bin/env bash
# Automated verification for GitLab #276 — pair-creation fee mitigates one-per-block griefing.
#
# Layers:
#   1. Contract unit/integration test (fee charge, treasury credit, gov setter, auth)
#   2. Full contract suite (regression)
#   3. Doc cross-links (security-model, contracts-terraclassic, contracts-security-audit F2)
#
# Refs: docs/security-model.md, docs/contracts-terraclassic.md, docs/contracts-security-audit.md (F2)
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
echo "  GitLab #276 — pair-creation fee (factory)"
echo "════════════════════════════════════════════════════════════════"

run_step "factory fee test (empty attach rejected, treasury credited, gov setter)" \
  bash -c 'cd smartcontracts && cargo test create_pair_charges_fee_to_treasury_and_gov_can_set_it --quiet'

run_step "one-per-block gate regression" \
  bash -c 'cd smartcontracts && cargo test test_create_pair_one_per_block_then_next_block_ok --quiet'

run_step "full contract suite" \
  make test-contracts

run_step "docs mention #276 / pair_creation_fee_uluna" \
  bash -c 'grep -q "#276" docs/security-model.md docs/contracts-terraclassic.md docs/contracts-security-audit.md'

run_step "local deploy sets pair_creation_fee_uluna=0" \
  bash -c 'grep -q "pair_creation_fee_uluna" scripts/deploy-dex-local.sh'

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
