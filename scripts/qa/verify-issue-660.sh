#!/usr/bin/env bash
# Automated verification for GitLab #660 — /pool Manage four peer actions.
#
# Proves (unit + docs; Playwright when the frontend can boot):
#   1. No page-level zap cards; four Manage tabs; no Advanced disclosure.
#   2. Zap add/withdraw are pair-scoped (no pair/LP picker that can select another pair).
#   3. Empty-pool copy points at Provide Liquidity.
#   4. Docs/skills M660-1–M660-8 + how-to no longer teach “two-sided is Advanced.”
#   5. C653: zap forms are card-glass wells, not nested shell-panel*.
#
# Refs: skills/AGENTS_FRONTEND_POOL_MANAGE_IA.md,
#       frontend-dapp/src/components/pool/PoolAdvancedManage.tsx,
#       docs/frontend.md § Pool Manage IA
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
echo "  GitLab #660 — /pool Manage four peer actions"
echo "════════════════════════════════════════════════════════════════"

# Dedicated Vite so parallel worktrees do not reuse another checkout on :3000/:5173.
export PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-30660}"
export PLAYWRIGHT_BASE_URL="http://127.0.0.1:${PLAYWRIGHT_WEB_PORT}"

run_step "frontend: PoolPage + howto copy + zap quote empty-pool" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/PoolPage.test.tsx \
    src/utils/__tests__/poolLpHowtoCopy.test.ts \
    src/utils/__tests__/oneSidedLiquidityQuote.test.ts \
    src/utils/__tests__/oneSidedLiquidityCopy.test.ts'

run_step "code: no page-level zap grid; Manage four tabs; no Advanced disclosure" \
  bash -c '! grep -qE "OneSidedAddCard|OneSidedWithdrawCard" frontend-dapp/src/pages/PoolPage.tsx &&
    ! grep -qE "md:grid-cols-2" frontend-dapp/src/pages/PoolPage.tsx &&
    grep -qE "pool-manage-tab-provide" frontend-dapp/src/components/pool/PoolAdvancedManage.tsx &&
    grep -qE "pool-manage-tab-withdraw" frontend-dapp/src/components/pool/PoolAdvancedManage.tsx &&
    grep -qE "pool-manage-tab-zap-add" frontend-dapp/src/components/pool/PoolAdvancedManage.tsx &&
    grep -qE "pool-manage-tab-zap-withdraw" frontend-dapp/src/components/pool/PoolAdvancedManage.tsx &&
    ! grep -qE "pool-card-advanced" frontend-dapp/src/components/pool/PoolAdvancedManage.tsx &&
    ! grep -rqE "ONE_SIDED_ADVANCED_LABEL" frontend-dapp/src/'

run_step "code: zap pair/LP implicit; forms are card-glass not nested shell-panel" \
  bash -c '! grep -qE "PairSearchSelect" frontend-dapp/src/components/pool/OneSidedAddCard.tsx &&
    grep -qE "pool-one-sided-lp-pinned" frontend-dapp/src/components/pool/OneSidedWithdrawCard.tsx &&
    ! grep -qE "MenuSelect" frontend-dapp/src/components/pool/OneSidedWithdrawCard.tsx &&
    grep -qE "card-glass" frontend-dapp/src/components/pool/OneSidedAddCard.tsx &&
    grep -qE "card-glass" frontend-dapp/src/components/pool/OneSidedWithdrawCard.tsx &&
    ! grep -qE "shell-panel" frontend-dapp/src/components/pool/OneSidedAddCard.tsx &&
    ! grep -qE "shell-panel" frontend-dapp/src/components/pool/OneSidedWithdrawCard.tsx &&
    grep -qE "Empty pool. Use Provide Liquidity" frontend-dapp/src/utils/oneSidedLiquidityQuote.ts &&
    grep -qE "pool-manage-empty-pool" frontend-dapp/src/components/pool/PoolAdvancedManage.tsx &&
    ! grep -qE "Use Advanced" frontend-dapp/src/utils/oneSidedLiquidityQuote.ts'

run_step "code: E2E helpers open Manage + tab, not pool-card-advanced" \
  bash -c 'grep -qE "openPoolManage" frontend-dapp/e2e/helpers/pool-ui.ts &&
    grep -qE "openFirstFactoryManage" frontend-dapp/e2e/helpers/pool-ui.ts &&
    ! grep -qE "getByTestId\\(.pool-card-advanced" frontend-dapp/e2e/helpers/pool-ui.ts &&
    test -f frontend-dapp/e2e/pool-manage-660.spec.ts &&
    grep -qE "GitLab #660" frontend-dapp/e2e/pool-manage-660.spec.ts'

run_step "docs: M660-1–M660-8 + how-to + glossary + user guide" \
  bash -c 'grep -qE "\*\*M660-1\*\*" docs/frontend.md &&
    grep -qE "\*\*M660-8\*\*" docs/frontend.md &&
    grep -qE "pool-manage-ia" docs/frontend.md &&
    grep -qE "Empty pool. Use Provide Liquidity" docs/frontend.md &&
    ! grep -qE "two-sided is \*\*Advanced\*\*" docs/frontend.md &&
    grep -qE "Zap Add" docs/user-lunc-liquidity.md &&
    ! grep -qE "Two-sided deposit is \*\*Advanced\*\*" docs/user-lunc-liquidity.md &&
    grep -qE "Zap Add / Zap Withdraw" docs/design-system.md &&
    grep -qE "verify-issue-660" docs/testing.md'

run_step "skill: AGENTS_FRONTEND_POOL_MANAGE_IA + crosslinks" \
  bash -c 'grep -qE "\*\*M660-1" skills/AGENTS_FRONTEND_POOL_MANAGE_IA.md &&
    grep -qE "\*\*M660-8" skills/AGENTS_FRONTEND_POOL_MANAGE_IA.md &&
    grep -qE "make verify-issue-660" skills/AGENTS_FRONTEND_POOL_MANAGE_IA.md &&
    grep -qE "AGENTS_FRONTEND_POOL_MANAGE_IA" skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md &&
    grep -qE "AGENTS_FRONTEND_POOL_MANAGE_IA" skills/AGENTS_FRONTEND_POOL_TABLE.md &&
    grep -qE "AGENTS_FRONTEND_POOL_MANAGE_IA" skills/AGENTS_FRONTEND_POOL_LP_HOWTO.md &&
    grep -qE "AGENTS_FRONTEND_POOL_MANAGE_IA" skills/AGENTS_FRONTEND_POOL_ZAP_FLOORS.md &&
    grep -qE "AGENTS_FRONTEND_POOL_MANAGE_IA" AGENTS.md &&
    grep -qE "verify-issue-660" Makefile'

if [[ "${VERIFY_ISSUE_660_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright T1–T6 / T13] skipped (VERIFY_ISSUE_660_SKIP_E2E=1)"
else
  run_step "playwright: Manage IA smoke (5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT}" PLAYWRIGHT_BASE_URL="${PLAYWRIGHT_BASE_URL}" bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test e2e/pool-manage-660.spec.ts e2e/pool-one-sided-533.spec.ts --project=e2e-smoke --workers=5'
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
