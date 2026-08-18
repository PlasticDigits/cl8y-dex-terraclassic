#!/usr/bin/env bash
# Automated verification for GitLab #559 — one-sided zap execution floors.
#
# Proves (unit + docs; Playwright P1–P3 / P9 when the stack can boot):
#   1. Zap-in provideAsk ≤ swapMinReturn (T-Z1–T-Z3 / AC1–AC3).
#   2. Zap-out swap/unwrap sized to min_assets + swapMinReturn (T-Z10–T-Z11 / AC4).
#   3. Human min-swap pre-sign (T-Z12 / AC7); conservative LP dust (T-Z7 / AC8).
#   4. Z559-1–Z559-4 docs + skill crosslinks; make verify-issue-533 still green.
#
# Refs: skills/AGENTS_FRONTEND_POOL_ZAP_FLOORS.md,
#       frontend-dapp/src/utils/oneSidedLiquidity.ts,
#       frontend-dapp/src/utils/oneSidedLiquidityQuote.ts
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

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
echo "  GitLab #559 — one-sided zap execution floors"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: zap math + tx + quote + copy floors (T-Z1–T-Z12)" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/oneSidedLiquidity.test.ts \
    src/utils/__tests__/oneSidedLiquidityTx.test.ts \
    src/utils/__tests__/oneSidedLiquidityQuote.test.ts \
    src/utils/__tests__/oneSidedLiquidityCopy.test.ts'

run_step "code: quoteOneSidedAdd provide amounts are floor-trimmed" \
  bash -c 'grep -qE "conservativeZapInProvide" frontend-dapp/src/utils/oneSidedLiquidityQuote.ts &&
    grep -qE "conservativeZapInProvide" frontend-dapp/src/utils/oneSidedLiquidity.ts &&
    grep -qE "conservativeZapOutExecution" frontend-dapp/src/utils/oneSidedLiquidity.ts &&
    grep -qE "provideAsk exceeds swap min_return" frontend-dapp/src/utils/oneSidedLiquidityTx.ts &&
    grep -qE "formatHumanMinSwapLine" frontend-dapp/src/utils/oneSidedLiquidityCopy.ts &&
    grep -qE "oneSidedAddPreSignAmountLines" frontend-dapp/src/components/pool/OneSidedAddCard.tsx &&
    grep -qE "conservativeZapOutExecution" frontend-dapp/src/components/pool/OneSidedWithdrawCard.tsx &&
    ! grep -nE "min swap \$\{snapshot.swapMinReturn\}" frontend-dapp/src/components/pool/OneSidedAddCard.tsx'

run_step "code: wrap/route-in still use floors; A18 order unchanged" \
  bash -c 'grep -qE "minimum_receive.*floors the zap amountIn" frontend-dapp/src/utils/oneSidedLiquidityQuote.ts &&
    grep -qE "Z559-1" frontend-dapp/src/utils/oneSidedLiquidityTx.ts &&
    grep -qE "slippage_tolerance: slippage" frontend-dapp/src/utils/oneSidedLiquidityTx.ts &&
    ! grep -qE "slippage_tolerance: null[,}]" frontend-dapp/src/utils/oneSidedLiquidityTx.ts'

run_step "docs: Z559-1–Z559-4 + frontend.md section" \
  bash -c 'grep -qE "\*\*Z559-1\*\*" docs/frontend.md &&
    grep -qE "\*\*Z559-4\*\*" docs/frontend.md &&
    grep -qE "pool-one-sided-zap-floors" docs/frontend.md &&
    grep -qE "quotes may be optimistic" docs/frontend.md &&
    grep -qE "make verify-issue-559" docs/frontend.md'

run_step "skill: AGENTS_FRONTEND_POOL_ZAP_FLOORS Z559 + verify-issue-559" \
  bash -c 'grep -qE "\*\*Z559-1" skills/AGENTS_FRONTEND_POOL_ZAP_FLOORS.md &&
    grep -qE "\*\*Z559-4" skills/AGENTS_FRONTEND_POOL_ZAP_FLOORS.md &&
    grep -qE "make verify-issue-559" skills/AGENTS_FRONTEND_POOL_ZAP_FLOORS.md &&
    grep -qE "AGENTS_FRONTEND_POOL_ZAP_FLOORS" AGENTS.md &&
    grep -qE "AGENTS_FRONTEND_POOL_ZAP_FLOORS" skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md &&
    grep -qE "verify-issue-559" Makefile'

run_step "crosslinks: one-sided + copy + wrap + slippage + howto" \
  bash -c 'grep -qE "AGENTS_FRONTEND_POOL_ZAP_FLOORS|#559" skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md &&
    grep -qE "AGENTS_FRONTEND_POOL_ZAP_FLOORS|#559" skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md &&
    grep -qE "AGENTS_FRONTEND_POOL_ZAP_FLOORS|#559" skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md &&
    grep -qE "AGENTS_FRONTEND_POOL_ZAP_FLOORS|#559" skills/AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md &&
    grep -qE "AGENTS_FRONTEND_POOL_ZAP_FLOORS|#559" skills/AGENTS_FRONTEND_POOL_LP_HOWTO.md &&
    grep -qE "AGENTS_FRONTEND_POOL_ZAP_FLOORS|#559" skills/AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md'

run_step "e2e: P9 conservative zap-in present (does not skip with PLAYWRIGHT_SKIP_CHAIN=1 alone)" \
  bash -c 'grep -qE "P9 conservative zap-in" frontend-dapp/e2e/pool-one-sided-533-tx.spec.ts &&
    grep -qE "GitLab #559" frontend-dapp/e2e/pool-one-sided-533-tx.spec.ts &&
    ! grep -qE "PLAYWRIGHT_SKIP_CHAIN=1" frontend-dapp/e2e/pool-one-sided-533-tx.spec.ts'

if [[ "${VERIFY_ISSUE_559_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright P1–P3] skipped (VERIFY_ISSUE_559_SKIP_E2E=1)"
else
  run_step "playwright: P1–P3 one-sided cards (e2e-smoke, 5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test e2e/pool-one-sided-533.spec.ts --project=e2e-smoke'
fi

if [[ "${VERIFY_ISSUE_559_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright P4–P9] skipped (VERIFY_ISSUE_559_SKIP_E2E=1)"
elif make has-localterra >/dev/null 2>&1 && [ -f frontend-dapp/.env.local ]; then
  run_step "playwright: P4–P9 one-sided tx including P9 floors (e2e-tx, 1 worker)" \
    bash -c 'CI=1 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test e2e/pool-one-sided-533-tx.spec.ts --project=e2e-tx'
else
  echo ""
  echo "[playwright P4–P9] skipped — LocalTerra or frontend-dapp/.env.local not ready"
  echo "  Provision: make setup-cloud-localterra"
fi

run_step "parent: verify-issue-533 still green (units + docs; skip nested e2e)" \
  bash -c 'VERIFY_ISSUE_533_SKIP_E2E=1 ./scripts/qa/verify-issue-533.sh'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  Results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for line in "${RESULTS[@]}"; do
  echo "  $line"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
