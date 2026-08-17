#!/usr/bin/env bash
# Verification for GitLab #537 — dApp fee-tier chrome gated on pair DISCOUNT_REGISTRY (I14).
#
# Layers (no LocalTerra required):
#   1. Pure helpers: advertisedDiscountBps / raw-key decode
#   2. LCD probe: raw state + smart-query fallback
#   3. Maker place hook: unwired vs wired vs mismatched registry
#   4. Swap / Pool fee chrome (strikethrough vs full fee_bps)
#   5. Docs/skills F537 + I14 crosslinks
#
# Refs: skills/AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md,
#       docs/frontend.md § pair-fee-tier-chrome,
#       docs/reference/fee-discount-tiers.md I14
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
echo "  GitLab #537 — pair-scoped CL8Y fee-tier chrome (I14)"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend unit: pairDiscountRegistry helpers + LCD probe + maker hook + Swap/Pool chrome" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/utils/__tests__/pairDiscountRegistry.test.ts \
    src/services/terraclassic/__tests__/pairDiscountRegistry.test.ts \
    src/hooks/__tests__/useLimitOrderMakerFeeRates.test.tsx \
    src/pages/PoolPage.feeDiscountRegistryBanner.test.tsx \
    src/pages/SwapPage.feeDiscountRegistryBanner.test.tsx'

run_step "code: Swap/Pool/Trade gate on pairDiscountApplies / getPairDiscountRegistry" \
  grep -qE 'useFeeDiscountRegistryStatus\(directPair' frontend-dapp/src/pages/SwapPage.tsx && \
  grep -qE 'pairDiscountApplies' frontend-dapp/src/pages/SwapPage.tsx && \
  grep -qE 'useFeeDiscountRegistryStatus\(pair.contract_addr\)' frontend-dapp/src/pages/PoolPage.tsx && \
  grep -qE 'pairDiscountApplies' frontend-dapp/src/pages/PoolPage.tsx && \
  grep -qE 'useFeeDiscountRegistryStatus\(pairAddr' frontend-dapp/src/components/pool/OneSidedAddCard.tsx && \
  grep -qE 'useFeeDiscountRegistryStatus\(pair\?\.contract_addr\)' frontend-dapp/src/components/pool/OneSidedWithdrawCard.tsx && \
  grep -qE 'getPairDiscountRegistry' frontend-dapp/src/hooks/useLimitOrderMakerFeeRates.ts && \
  grep -qE 'pairDiscountApplies' frontend-dapp/src/hooks/useLimitOrderMakerFeeRates.ts

run_step "code: fail-closed advertisedDiscountBps; raw key constant" \
  grep -qE 'export function advertisedDiscountBps' frontend-dapp/src/utils/pairDiscountRegistry.ts && \
  grep -qE 'PAIR_DISCOUNT_REGISTRY_STORAGE_KEY = .discount_registry' frontend-dapp/src/utils/pairDiscountRegistry.ts && \
  grep -qE 'pairDiscountRegistryRawKeyB64' frontend-dapp/src/utils/pairDiscountRegistry.ts && \
  grep -qE 'queryContractRaw' frontend-dapp/src/services/terraclassic/pairDiscountRegistry.ts && \
  grep -qE 'get_discount_registry' frontend-dapp/src/services/terraclassic/pairDiscountRegistry.ts

run_step "docs: I14 + F537-1–F537-6 + GetDiscountRegistry live-wasm note" \
  grep -qE '\*\*I14\*\*' docs/reference/fee-discount-tiers.md && \
  grep -qE 'pair-fee-tier-chrome' docs/frontend.md && \
  grep -qE '\*\*F537-1\*\*' docs/frontend.md && \
  grep -qE '\*\*F537-6\*\*' docs/frontend.md && \
  grep -qE 'is implemented on pair \*\*1.14.0\*\*' docs/contracts-terraclassic.md && \
  grep -qE '#537' docs/integrators.md

run_step "skill: AGENTS_FRONTEND_PAIR_FEE_DISCOUNT + fee-tier / limit-price / AGENTS.md crosslinks" \
  grep -qE 'make verify-issue-537' skills/AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md && \
  grep -qE 'F537-1' skills/AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md && \
  grep -qE 'AGENTS_FRONTEND_PAIR_FEE_DISCOUNT|#537' skills/AGENTS_FEE_DISCOUNT_TIERS.md && \
  grep -qE 'AGENTS_FRONTEND_PAIR_FEE_DISCOUNT|#537' skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md && \
  grep -qE 'AGENTS_FRONTEND_PAIR_FEE_DISCOUNT' AGENTS.md

echo ""
echo "── retest ──"
run_step "retest frontend unit" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/utils/__tests__/pairDiscountRegistry.test.ts \
    src/services/terraclassic/__tests__/pairDiscountRegistry.test.ts \
    src/hooks/__tests__/useLimitOrderMakerFeeRates.test.tsx \
    src/pages/PoolPage.feeDiscountRegistryBanner.test.tsx \
    src/pages/SwapPage.feeDiscountRegistryBanner.test.tsx'
run_step "retest docs I14" \
  grep -qE '\*\*I14\*\*' docs/reference/fee-discount-tiers.md

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
