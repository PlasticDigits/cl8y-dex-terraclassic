#!/usr/bin/env bash
# Automated verification for GitLab #313 — factory pair-creation BankMsg::Send review.
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
echo "  GitLab #313 — factory treasury bank send review"
echo "════════════════════════════════════════════════════════════════"

run_step "adversarial fee paths (stray denom, overpay, treasury rotation)" \
  bash -c 'cd smartcontracts && cargo test create_pair_fee_bank_send_adversarial_paths --quiet'

run_step "#276 fee regression tests" \
  bash -c 'cd smartcontracts && cargo test create_pair_charges_fee_to_treasury_and_gov_can_set_it --quiet && cargo test create_pair_refunds_uluna_when_fee_disabled --quiet'

run_step "audit doc present" \
  test -f docs/audits/factory-treasury-bank-send.md

run_step "full contract suite" \
  make test-contracts

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
