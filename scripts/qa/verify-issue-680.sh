#!/usr/bin/env bash
# Automated verification for GitLab #680 — /charts UST1/USD hero + ?price= + page-wide invert.
#
# Proves (unit + docs; no chain required):
#   1. ?price= parser allowlist / hostile ignore (last key wins).
#   2. Charts default ≠ Trade T524-3; storage prefixes isolated.
#   3. Display OHLC uses invertUsd + High/Low swap; % recomputed.
#   4. Hero pick prefers UST1/cUSTC; Trade firstCatalogPairAddress unchanged.
#   5. ChartsPage C680-1–C680-8 Vitest + V5 volume unchanged.
#   6. Docs/skills C680-1–C680-9; C543-3 Trade-only; chrome nesting green.
#
# Refs: skills/AGENTS_FRONTEND_CHARTS_UST1_HERO.md,
#       docs/frontend.md § Charts UST1/USD hero
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
echo "  GitLab #680 — Charts UST1/USD hero + ?price="
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: route / orientation / OHLC / hero / ChartsPage" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/chartsPairRoute.test.ts \
    src/utils/__tests__/tradePairDisplayOrientation.test.ts \
    src/utils/__tests__/pairPriceUsd.test.ts \
    src/utils/__tests__/chartsPairStats.test.ts \
    src/utils/__tests__/pairCatalogRank.test.ts \
    src/pages/ChartsPage.test.tsx'

run_step "frontend: Trade #524 other-side default preserved" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/TradePage.test.tsx -t "524"'

run_step "code: Charts hook + storage prefix isolation" \
  bash -c '
    set -euo pipefail
    grep -q "useChartsPairDisplayOrientation" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "cl8y-dex-charts-pair-invert:" frontend-dapp/src/utils/tradePairDisplayOrientation.ts
    grep -q "CHARTS_PAIR_DISPLAY_INVERT_STORAGE_PREFIX" frontend-dapp/src/utils/tradePairDisplayOrientation.ts
    grep -q "resolveDisplayPairStatsUsdOhlc" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "parseChartsPriceQuery" frontend-dapp/src/utils/chartsPairRoute.ts
    grep -q "MAINNET_UST1_CUSTC_PAIR_ADDRESS" frontend-dapp/src/utils/ust1SecondaryMarket.ts
    grep -q "resolveChartsHeroPairAddress" frontend-dapp/src/utils/pairCatalogRank.ts
    ! grep -q "writeStoredPairDisplayInverted" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -q "getOverview" frontend-dapp/src/pages/ChartsPage.tsx
  '

run_step "docs: C680 invariants + Trade-only C543-3" \
  bash -c '
    set -euo pipefail
    grep -q "charts-ust1-usd-hero" docs/frontend.md
    grep -qE "\*\*C680-1\*\*" docs/frontend.md
    grep -qE "\*\*C680-9\*\*" docs/frontend.md
    grep -q "Trade-only" docs/frontend.md
    grep -q "cl8y-dex-charts-pair-invert:" docs/frontend.md
    grep -q "Charts UST1/USD hero (#680)" docs/indexer-invariants.md
    grep -q "verify-issue-680" docs/testing.md
  '

run_step "skill: AGENTS_FRONTEND_CHARTS_UST1_HERO + crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_CHARTS_UST1_HERO.md
    grep -qE "\*\*C680-1" skills/AGENTS_FRONTEND_CHARTS_UST1_HERO.md
    grep -qE "\*\*C680-9" skills/AGENTS_FRONTEND_CHARTS_UST1_HERO.md
    grep -q "make verify-issue-680" skills/AGENTS_FRONTEND_CHARTS_UST1_HERO.md
    grep -qE "AGENTS_FRONTEND_CHARTS_UST1_HERO|#680" skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md
    grep -qE "AGENTS_FRONTEND_CHARTS_UST1_HERO|#680" skills/AGENTS_FRONTEND_USD_CANDLE_INVERT.md
    grep -qE "AGENTS_FRONTEND_CHARTS_UST1_HERO|#680" skills/AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md
    grep -qE "AGENTS_FRONTEND_CHARTS_UST1_HERO|#680" skills/AGENTS_FRONTEND_CHARTS_PAIR_STATS.md
    grep -q "AGENTS_FRONTEND_CHARTS_UST1_HERO" AGENTS.md
    grep -q "verify-issue-680" AGENTS.md
    grep -q "verify-issue-680" Makefile
  '

run_step "guard: check_chrome_nesting.py" \
  python3 scripts/check_chrome_nesting.py

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
echo "==> GitLab #680 verification passed"
