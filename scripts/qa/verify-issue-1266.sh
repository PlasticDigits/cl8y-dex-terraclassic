#!/usr/bin/env bash
# Automated verification for #1266 — /charts Select Pair first change vs idle hero.
#
# Proves (unit + docs; no chain required):
#   1. Hero auto-pick is idle-only (shouldAutoPickChartsHeroPair).
#   2. Catalog-head fallback does not replace a valid bech32.
#   3. ChartsPage RTL clicks #chart-pair-select (no remount-only switch).
#   4. Docs/skill C1266-1–C1266-8; hero skill must not override selectPair.
#   5. C680 subset + chrome nesting stay green.
#
# Refs: skills/AGENTS_FRONTEND_CHARTS_PAIR_SELECT.md,
#       docs/frontend.md § Charts Select Pair
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
echo "  #1266 — Charts Select Pair first change sticks"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: chartsPairRoute idle-hero guards + ChartsPage MenuSelect" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/chartsPairRoute.test.ts \
    src/pages/ChartsPage.test.tsx'

run_step "code: selectPair / idle hero / href-only navigate" \
  bash -c '
    set -euo pipefail
    grep -q "shouldAutoPickChartsHeroPair" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "shouldSnapChartsSelectionToCatalogHead" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "userCommittedPairRef" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "isChartsPairRouteParam(addr)" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "chartsPairHref" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "replace: true" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "shouldAutoPickChartsHeroPair" frontend-dapp/src/utils/chartsPairRoute.ts
    grep -q "shouldSnapChartsSelectionToCatalogHead" frontend-dapp/src/utils/chartsPairRoute.ts
    ! grep -q "cl8y-dex-charts-last-pair" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -q "localStorage.setItem" frontend-dapp/src/pages/ChartsPage.tsx
  '

run_step "docs: C1266 invariants + skill grep that hero must not override selectPair" \
  bash -c '
    set -euo pipefail
    grep -q "charts-select-pair" docs/frontend.md
    grep -qE "\*\*C1266-1\*\*" docs/frontend.md
    grep -qE "\*\*C1266-8\*\*" docs/frontend.md
    grep -q "must not override \`selectPair\`" docs/frontend.md
    grep -q "Charts Select Pair first change (#1266)" docs/indexer-invariants.md
    grep -q "verify-issue-1266" docs/testing.md
    test -f skills/AGENTS_FRONTEND_CHARTS_PAIR_SELECT.md
    grep -qE "\*\*C1266-1" skills/AGENTS_FRONTEND_CHARTS_PAIR_SELECT.md
    grep -qE "\*\*C1266-8" skills/AGENTS_FRONTEND_CHARTS_PAIR_SELECT.md
    grep -q "must not override" skills/AGENTS_FRONTEND_CHARTS_UST1_HERO.md
    grep -q "selectPair" skills/AGENTS_FRONTEND_CHARTS_UST1_HERO.md
    grep -q "AGENTS_FRONTEND_CHARTS_PAIR_SELECT" AGENTS.md
    grep -q "verify-issue-1266" AGENTS.md
    grep -q "verify-issue-1266" Makefile
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
echo "==> #1266 verification passed"
