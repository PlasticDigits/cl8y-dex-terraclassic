#!/usr/bin/env bash
# Verification for GitLab #514 — limit-order placement discount shift.
#
# Refs: docs/reference/fee-discount-tiers.md (I13)
#       skills/AGENTS_FEE_DISCOUNT_TIERS.md
#       scripts/upgrade-514-limit-discount.sh
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
echo "  GitLab #514 — limit placement discount (swap/take unchanged)"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_dex_common() {
  (cd smartcontracts && cargo test -p dex-common fee_discount -- --quiet)
}

run_integration() {
  (cd smartcontracts && cargo test -p cl8y-dex-tests --lib \
    limit_placement_shifted_discount_swap_fee_unchanged_514 -- --quiet) &&
  (cd smartcontracts && cargo test -p cl8y-dex-tests --lib test_query_tiers -- --quiet)
}

run_docs() {
  set -euo pipefail
  python3 scripts/check_fee_discount_tier_docs.py
  rg -q "I13" docs/reference/fee-discount-tiers.md
  rg -q "limit_discount_bps" docs/reference/fee-discount-tiers.md
  rg -q "limit_discount_bps" docs/contracts-terraclassic.md
  rg -q "I13" skills/AGENTS_FEE_DISCOUNT_TIERS.md
  rg -q "verify-issue-514" AGENTS.md
  rg -q "upgrade-514-limit-discount" skills/AGENTS_FEE_DISCOUNT_TIERS.md
  test -f scripts/upgrade-514-limit-discount.sh
  rg -q "limit_discount_bps" smartcontracts/contracts/fee-discount/src/state.rs
  rg -q "limit_effective_fee_bps_from_discount" smartcontracts/contracts/pair/src/discount_cache.rs
}

run_frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/utils/__tests__/limitOrderFeeSummary.test.ts
}

run_upgrade_script() {
  set -euo pipefail
  chmod +x scripts/upgrade-514-limit-discount.sh
  bash -n scripts/upgrade-514-limit-discount.sh
  DRY_RUN=1 UPGRADE514_SKIP_STORE=1 UPGRADE514_SKIP_MIGRATE=1 \
    UPGRADE514_PAIR_CODE_ID=10 UPGRADE514_FEE_DISCOUNT_CODE_ID=11 \
    UPGRADE514_FACTORY_ADDRESS=terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea \
    UPGRADE514_FEE_DISCOUNT_ADDRESS=terra1feediscount514placeholder000000000000000000000000 \
    ./scripts/upgrade-514-limit-discount.sh
}

echo ""
echo "── first pass ──"
run_step "dex-common: limit discount shift + 180 bps targets" run_dex_common
run_step "integration: T9 place 0 / swap 9 + GetTiers column" run_integration
run_step "docs: I13 + skill + drift guard" run_docs
run_step "frontend: resolveLimitDiscountBps" run_frontend
run_step "upgrade script: bash -n + DRY_RUN" run_upgrade_script

echo ""
echo "── retest ──"
run_step "retest dex-common" run_dex_common
run_step "retest integration" run_integration
run_step "retest docs drift" run_docs
run_step "retest frontend" run_frontend

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  $PASS passed, $FAIL failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "OK: #514 limit-order placement discount"
