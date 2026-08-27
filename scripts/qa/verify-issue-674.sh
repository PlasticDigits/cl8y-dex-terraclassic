#!/usr/bin/env bash
# Automated verification for GitLab #674 — hide test-gem performance on /portfolio.
#
# Proves (unit + docs; Playwright when LocalTerra is up):
#   1. Classifier reuses #534 isTestPair / columbus-5 gem addrs (no second list).
#   2. Default hide of gem Open Positions + recent activity; toggle reveals + divider.
#   3. Toggle absent when every row is economic; /trader table stays unfiltered.
#   4. Docs/skills P674-1–P674-8 crosslinked; P562-7 points at #674.
#
# Refs: skills/AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS.md,
#       frontend-dapp/src/utils/portfolioPerformanceFilter.ts,
#       docs/frontend.md § My Portfolio
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
echo "  GitLab #674 — hide test-gem performance on /portfolio"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: filter helper + portfolio + positions table (P674-1–P674-6)" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/portfolioPerformanceFilter.test.ts \
    src/pages/PortfolioPage.test.tsx \
    src/components/trader/TraderPositionsTable.test.tsx'

run_step "code: portfolio uses shared classifier + toggle (no second gem list)" \
  bash -c 'grep -qE "visiblePortfolioPositions" frontend-dapp/src/pages/PortfolioPage.tsx &&
    grep -qE "visiblePortfolioTrades" frontend-dapp/src/pages/PortfolioPage.tsx &&
    grep -qE "PortfolioShowTestPairsToggle" frontend-dapp/src/pages/PortfolioPage.tsx &&
    grep -qE "isTestPair" frontend-dapp/src/utils/portfolioPerformanceFilter.ts &&
    grep -qE "isGemTokenId" frontend-dapp/src/utils/portfolioPerformanceFilter.ts &&
    grep -qE "showTestPairDivider" frontend-dapp/src/components/trader/TraderPositionsTable.tsx &&
    ! grep -qE "EMBER|CORAL|JADE" frontend-dapp/src/utils/portfolioPerformanceFilter.ts &&
    ! grep -qE "showGems" frontend-dapp/src/pages/PortfolioPage.tsx frontend-dapp/src/utils/portfolioPerformanceFilter.ts'

run_step "code: /trader does not default-hide gems (P674-5)" \
  bash -c 'grep -qE "TraderPositionsTable" frontend-dapp/src/pages/TraderPage.tsx &&
    ! grep -qE "visiblePortfolioPositions|showTestPairs|PortfolioShowTestPairsToggle" frontend-dapp/src/pages/TraderPage.tsx'

run_step "docs: frontend.md P674-1–P674-8 + indexer-invariants row" \
  bash -c 'grep -qE "Hide test-gem performance \\(#674\\)" docs/frontend.md &&
    grep -qE "portfolio-hide-test-gems" docs/frontend.md &&
    grep -qE "\\*\\*P674-1\\*\\*" docs/frontend.md &&
    grep -qE "\\*\\*P674-8\\*\\*" docs/frontend.md &&
    grep -qE "Portfolio hide test-gem performance \\(#674\\)" docs/indexer-invariants.md &&
    grep -qE "P674-1–P674-8" docs/indexer-invariants.md &&
    grep -qE "P562-7" docs/frontend.md &&
    grep -qE "#674" docs/frontend.md'

run_step "skill: AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS + verify-issue-674" \
  bash -c 'grep -qE "\\*\\*P674-1" skills/AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS.md &&
    grep -qE "\\*\\*P674-8" skills/AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS.md &&
    grep -qE "make verify-issue-674" skills/AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS.md &&
    grep -qE "portfolioPerformanceFilter" skills/AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS.md'

run_step "crosslinks: portfolio + P&L + #534 + #562 + AGENTS.md + testing.md" \
  bash -c 'grep -qE "AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS|#674" skills/AGENTS_FRONTEND_PORTFOLIO.md &&
    grep -qE "AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS|#674" skills/AGENTS_FRONTEND_PORTFOLIO_PNL.md &&
    grep -qE "AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS|#674" skills/AGENTS_FRONTEND_HUB_PNL.md &&
    grep -qE "AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS|#674" skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md &&
    grep -qE "AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS|#674" skills/AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md &&
    grep -qE "AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS|#674" AGENTS.md &&
    grep -qE "verify-issue-674" AGENTS.md &&
    grep -qE "verify-issue-674" docs/testing.md &&
    grep -qE "P674-1" docs/testing.md'

if make -s has-localterra >/dev/null 2>&1; then
  run_step "playwright: portfolio shell + no forced test-pairs toggle (5 workers)" \
    bash -c 'PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-3174}" bash scripts/with-node.sh --cwd frontend-dapp -- \
      ./node_modules/.bin/playwright test --project=e2e-smoke e2e/portfolio.spec.ts -g "674"'
else
  echo ""
  echo "[playwright: portfolio shell + no forced test-pairs toggle (5 workers)]"
  echo "  SKIP (LocalTerra not up — unit + docs still required)"
  ok "playwright: skipped (no LocalTerra)"
fi

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  $PASS passed, $FAIL failed"
echo "────────────────────────────────────────────────────────────────"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
