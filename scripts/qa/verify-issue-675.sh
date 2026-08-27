#!/usr/bin/env bash
# Verification for GitLab #675: portfolio/trader unrealized P&L (hub mark vs on-DEX cost).
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
echo "  GitLab #675 — portfolio/trader unrealized P&L"
echo "════════════════════════════════════════════════════════════════"

run_step "docs: invariants + skill + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_PORTFOLIO_UNREALIZED.md
    grep -q "Portfolio unrealized P&amp;L (#675)" docs/indexer-invariants.md
    grep -q "P675-1" skills/AGENTS_FRONTEND_PORTFOLIO_UNREALIZED.md
    grep -q "P675-8" skills/AGENTS_FRONTEND_PORTFOLIO_UNREALIZED.md
    grep -q "AGENTS_FRONTEND_PORTFOLIO_UNREALIZED" AGENTS.md
    grep -q "verify-issue-675" AGENTS.md
    grep -q "P675" docs/frontend.md
    grep -q "No cost basis" docs/frontend.md
    grep -q "sumUnrealizedPnlUsd" docs/indexer-invariants.md
    grep -q "AGENTS_FRONTEND_PORTFOLIO_UNREALIZED" skills/AGENTS_FRONTEND_PORTFOLIO.md
    grep -q "AGENTS_FRONTEND_PORTFOLIO_UNREALIZED" skills/AGENTS_FRONTEND_PORTFOLIO_PNL.md
    grep -q "AGENTS_FRONTEND_PORTFOLIO_UNREALIZED" skills/AGENTS_FRONTEND_HUB_PNL.md
    grep -q "AGENTS_FRONTEND_PORTFOLIO_UNREALIZED" skills/AGENTS_INDEXER_HUB_USD.md
    grep -q "verify-issue-675" docs/testing.md
    grep -q "#675" indexer/src/indexer/position_tracker.rs
  '

run_step "source: mark uses hub helper; no quoteTokenUsd pegs" \
  bash -c '
    set -euo pipefail
    if grep -nF "quoteTokenUsd" frontend-dapp/src/utils/traderPositionDisplay.ts; then
      echo "traderPositionDisplay.ts must not call quoteTokenUsd (\$1 / 2.5x pegs)" >&2
      exit 1
    fi
    grep -q "sumUnrealizedPnlUsd" frontend-dapp/src/utils/traderPositionDisplay.ts
    grep -q "NO_COST_BASIS_LABEL" frontend-dapp/src/utils/traderPositionDisplay.ts
    grep -q "positionHasOnDexCostBasis" frontend-dapp/src/utils/traderPositionDisplay.ts
    grep -q "useTraderUsdMarks" frontend-dapp/src/hooks/useTraderUsdMarks.ts
    grep -q "trader-position-mark" frontend-dapp/src/components/trader/TraderPositionsTable.tsx
    grep -q "trader-position-unrealized" frontend-dapp/src/components/trader/TraderPositionsTable.tsx
    grep -q "trader-summary-unrealized-pnl" frontend-dapp/src/components/trader/TraderSummaryStats.tsx
    grep -q "No cost basis" frontend-dapp/src/utils/traderPositionDisplay.ts
  '

run_step "frontend: mark helper + positions table + summary + portfolio" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/utils/__tests__/traderPositionDisplay.test.ts src/components/trader/TraderPositionsTable.test.tsx src/components/trader/TraderSummaryStats.test.tsx src/pages/PortfolioPage.test.tsx'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
