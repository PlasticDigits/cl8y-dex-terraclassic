#!/usr/bin/env bash
# Verification for GitLab #560: portfolio/trader realized P&L USD from hub prices.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env; no wasm deploy).
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
echo "  GitLab #560 — portfolio/trader P&L USD from hub prices"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs: invariants + skill + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_HUB_PNL.md
    grep -q "Portfolio hub P&amp;L USD (#560)" docs/indexer-invariants.md
    grep -q "P560-1" skills/AGENTS_FRONTEND_HUB_PNL.md
    grep -q "P560-6" skills/AGENTS_FRONTEND_HUB_PNL.md
    grep -q "AGENTS_FRONTEND_HUB_PNL" AGENTS.md
    grep -q "verify-issue-560" AGENTS.md
    grep -q "hub-prices" docs/frontend.md
    grep -q "P560" docs/frontend.md
    grep -q "traderUsdMarksFromHub" skills/AGENTS_FRONTEND_PORTFOLIO_PNL.md
    grep -q "AGENTS_FRONTEND_HUB_PNL" skills/AGENTS_INDEXER_HUB_USD.md
    grep -q "P560-1" docs/runbooks/indexer-external-oracle.md
  '

run_step "source: trader P&L does not use quoteTokenUsd pegs; CEX ustr unused" \
  bash -c '
    set -euo pipefail
    if grep -nF "quoteTokenUsd" frontend-dapp/src/utils/traderPositionDisplay.ts; then
      echo "traderPositionDisplay.ts must not call quoteTokenUsd (\$1 / 2.5x pegs)" >&2
      exit 1
    fi
    if grep -n "getOraclePrice" frontend-dapp/src/components/trader/TraderSummaryStats.tsx | grep -q ustr; then
      echo "TraderSummaryStats must not fetch CEX /oracle/price/ustr" >&2
      exit 1
    fi
    grep -q "traderUsdMarksFromHub" frontend-dapp/src/utils/traderPositionDisplay.ts
    grep -q "useProtocolHubPricesQuery" frontend-dapp/src/components/trader/TraderSummaryStats.tsx
    grep -q "TRADER_PNL_EM_DASH" frontend-dapp/src/components/trader/PnlValue.tsx
    if grep -nF "N/A" frontend-dapp/src/components/trader/PnlValue.tsx; then
      echo "PnlValue must show em dash, not N/A" >&2
      exit 1
    fi
    grep -q "map_trader_position" indexer/src/api/traders.rs
  '

run_step "indexer lib: missing-asset position mapper" \
  bash -c 'cd indexer && cargo test --lib position_map_tests -- --quiet'

run_step "indexer integration: positions still expose decimals" \
  bash -c 'cd indexer && cargo test --test api_traders get_trader_positions_returns_rows -- --test-threads=1 --quiet'

run_step "frontend: hub P&L helper + summary + positions + portfolio" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/utils/__tests__/traderPositionDisplay.test.ts src/components/trader/TraderPositionsTable.test.tsx src/components/trader/TraderSummaryStats.test.tsx src/pages/PortfolioPage.test.tsx'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
