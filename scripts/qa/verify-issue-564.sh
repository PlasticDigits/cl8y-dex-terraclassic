#!/usr/bin/env bash
# Verification for GitLab #564: Charts pair 24h Stats + TWAP human scale.
#
# Frontend-only (no chain). Indexer JSON units must stay raw.
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
echo "  GitLab #564 — Charts pair 24h Stats + TWAP human scale"
echo "════════════════════════════════════════════════════════════════"

run_step "docs: invariants + skill + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_CHARTS_PAIR_STATS.md
    grep -q "S564-1" docs/frontend.md
    grep -q "charts-pair-24h-stats" docs/frontend.md
    grep -q "AGENTS_FRONTEND_CHARTS_PAIR_STATS" AGENTS.md
    grep -q "Charts pair 24h Stats (#564)" docs/indexer-invariants.md
    grep -q "arithmetic" docs/twap-oracle.md
    grep -q "price_a_cumulatives" docs/twap-oracle.md
    ! grep -q "tick_cumulatives" docs/twap-oracle.md
    grep -q "verify-issue-564" Makefile
    grep -q "AGENTS_FRONTEND_CHARTS_PAIR_STATS" skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md
  '

run_step "code: Charts does not formatNum raw volume or TWAP" \
  bash -c '
    set -euo pipefail
    grep -q "formatPairStatsVolume" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "formatTwapHumanPrice" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "charts-pair-vol-base" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "charts-pair-vol-usd" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "volume_usd" frontend-dapp/src/types/index.ts
    ! grep -qE "formatNum\(stats\.volume_base" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -qE "formatNum\(stats\.volume_quote" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -qE "formatNum\(entry\.price" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "volumeScale" frontend-dapp/src/components/charts/PriceChart.tsx
    grep -q "quoteDecimals" frontend-dapp/src/components/charts/priceChartCandles.ts
    ! grep -q "human_volume" indexer/src/api/pairs.rs
    grep -q "pub volume_base: String" indexer/src/api/pairs.rs
    grep -q "pub volume_quote: String" indexer/src/api/pairs.rs
  '

run_step "frontend: formatters + ChartsPage + histogram" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/formatAmount.test.ts \
    src/utils/__tests__/chartsPairStats.test.ts \
    src/services/terraclassic/__tests__/oracle.test.ts \
    src/pages/ChartsPage.test.tsx \
    src/components/charts/__tests__/priceChartCandles.test.ts'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
