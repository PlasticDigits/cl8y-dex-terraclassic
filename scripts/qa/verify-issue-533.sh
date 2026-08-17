#!/usr/bin/env bash
# Automated verification for GitLab #533 — one-sided pool add/withdraw (auto zap + wrap).
#
# Proves (unit + docs; Playwright P1–P3 when the frontend can boot):
#   1. Zap-in/out math (T1–T8) + tx floors (T9–T10 / A7 / A12).
#   2. Retail add/withdraw cards; Advanced two-sided; LP decimals 18.
#   3. H531-3 / Z533-1–Z533-10 docs + skill crosslinks.
#
# Refs: skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md,
#       frontend-dapp/src/utils/oneSidedLiquidity.ts,
#       frontend-dapp/src/components/pool/OneSidedAddCard.tsx
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
echo "  GitLab #533 — one-sided pool add/withdraw"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: zap math + tx + quote + howto + PoolPage" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/oneSidedLiquidity.test.ts \
    src/utils/__tests__/oneSidedLiquidityTx.test.ts \
    src/utils/__tests__/oneSidedLiquidityQuote.test.ts \
    src/utils/__tests__/poolLpHowtoCopy.test.ts \
    src/utils/__tests__/maxSpendableAmount.test.ts \
    src/pages/PoolPage.test.tsx \
    src/services/terraclassic/__tests__/transactions.test.ts'

run_step "code: retail cards + Advanced + LP 18 + no null slippage on zap builder" \
  bash -c 'grep -qE "pool-one-sided-add" frontend-dapp/src/components/pool/OneSidedAddCard.tsx &&
    grep -qE "pool-one-sided-withdraw" frontend-dapp/src/components/pool/OneSidedWithdrawCard.tsx &&
    grep -qE "pool-card-advanced" frontend-dapp/src/pages/PoolPage.tsx &&
    grep -qE "PAIR_LP_CW20_DECIMALS" frontend-dapp/src/utils/oneSidedLiquidity.ts &&
    grep -qE "slippage_tolerance: slippage" frontend-dapp/src/utils/oneSidedLiquidityTx.ts &&
    ! grep -qE "slippage_tolerance: null[,}]" frontend-dapp/src/utils/oneSidedLiquidityTx.ts'

run_step "code: unwrap uses quoted amount (A7) not wallet-balance unwrap in zap-out builder" \
  bash -c 'grep -qE "unwrapAmountMatchesQuote" frontend-dapp/src/utils/oneSidedLiquidityTx.ts &&
    ! grep -nE "getTokenBalance\\(" frontend-dapp/src/utils/oneSidedLiquidityTx.ts'

run_step "docs: Z533-1–Z533-10 + H531-3 one-sided + user guide" \
  bash -c 'grep -qE "\*\*Z533-1\*\*" docs/frontend.md &&
    grep -qE "\*\*Z533-10\*\*" docs/frontend.md &&
    grep -qE "Retail Add is \*\*one-sided\*\*" docs/frontend.md &&
    grep -qE "one token" docs/user-lunc-liquidity.md &&
    grep -qE "pool-one-sided-liquidity" docs/user-lunc-liquidity.md'

run_step "skill: AGENTS_FRONTEND_POOL_ONE_SIDED Z533 + verify-issue-533" \
  bash -c 'grep -qE "\*\*Z533-1" skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md &&
    grep -qE "\*\*Z533-10" skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md &&
    grep -qE "make verify-issue-533" skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md &&
    grep -qE "one-sided" skills/AGENTS_FRONTEND_POOL_LP_HOWTO.md &&
    grep -qE "AGENTS_FRONTEND_POOL_ONE_SIDED" AGENTS.md'

run_step "crosslinks: howto + provide preview + copy + wrap + slippage + AGENTS.md" \
  bash -c 'grep -qE "AGENTS_FRONTEND_POOL_ONE_SIDED|#533" skills/AGENTS_FRONTEND_POOL_LP_HOWTO.md &&
    grep -qE "AGENTS_FRONTEND_POOL_ONE_SIDED|#533" skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md &&
    grep -qE "AGENTS_FRONTEND_POOL_ONE_SIDED|#533" skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md &&
    grep -qE "AGENTS_FRONTEND_POOL_ONE_SIDED|#533" skills/AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md &&
    grep -qE "verify-issue-533" Makefile'

if [[ "${VERIFY_ISSUE_533_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright P1–P3] skipped (VERIFY_ISSUE_533_SKIP_E2E=1)"
else
  run_step "playwright: P1–P3 one-sided cards (e2e-smoke, 5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test e2e/pool-one-sided-533.spec.ts --project=e2e-smoke'
fi

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
