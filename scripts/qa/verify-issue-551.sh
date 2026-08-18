#!/usr/bin/env bash
# Verification for GitLab #551: portfolio/trader P&L human scale + USD totals.
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
echo "  GitLab #551 — portfolio/trader P&L human scale"
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
    test -f skills/AGENTS_FRONTEND_PORTFOLIO_PNL.md
    grep -q "Trader positions human scale (#551)" docs/indexer-invariants.md
    grep -q "P551-1" docs/indexer-invariants.md
    grep -q "P551-5" skills/AGENTS_FRONTEND_PORTFOLIO_PNL.md
    grep -q "AGENTS_FRONTEND_PORTFOLIO_PNL" AGENTS.md
    grep -q "P551-1" docs/frontend.md
    grep -q "formatScaledPosition" skills/AGENTS_FRONTEND_PORTFOLIO.md
  '

run_step "indexer lib: position_tracker human avg + clamp" \
  bash -c 'cd indexer && cargo test --lib position_tracker -- --quiet'

run_step "indexer integration: positions expose decimals" \
  bash -c 'cd indexer && cargo test --test api_traders get_trader_positions_returns_rows -- --test-threads=1 --quiet'

run_step "frontend: scale helper + positions table + summary + portfolio" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/utils/__tests__/traderPositionDisplay.test.ts src/components/trader/TraderPositionsTable.test.tsx src/components/trader/TraderSummaryStats.test.tsx src/pages/PortfolioPage.test.tsx'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
