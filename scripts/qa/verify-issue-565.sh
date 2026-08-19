#!/usr/bin/env bash
# Verification for GitLab #565: Charts pair 24h Vol (USD) + human token remainder.
#
# Frontend-only (no chain / Postgres required).
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
echo "  GitLab #565 — Charts pair 24h Vol (USD)"
echo "════════════════════════════════════════════════════════════════"

run_step "docs: invariants + skill + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_CHARTS_PAIR_STATS.md
    grep -q "Charts pair 24h volume (#565)" docs/indexer-invariants.md
    grep -q "AGENTS_FRONTEND_CHARTS_PAIR_STATS" AGENTS.md
    grep -q "charts-pair-volume-usd" docs/frontend.md
    grep -q "charts-pair-stats" docs/frontend.md
    grep -q "P565-1" skills/AGENTS_FRONTEND_CHARTS_PAIR_STATS.md
    grep -q "P565-7" skills/AGENTS_FRONTEND_CHARTS_PAIR_STATS.md
    grep -q "verify-issue-565" Makefile
    grep -q "#565" skills/AGENTS_FRONTEND_CHARTS_OVERVIEW.md
    grep -q "#565" skills/AGENTS_FRONTEND_TRADER_VOLUME_USD.md
    grep -q "#565" skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md
    grep -q "#565" skills/AGENTS_INDEXER_PAIR_PRICE_USD.md
    grep -q "#565" docs/integrators-hybrid-volume.md
    ! grep -q "Pair-level \`Vol (token)\` on the same page is \*\*out of scope\*\*" docs/frontend.md
  '

run_step "code: Charts does not formatNum raw pair volume" \
  bash -c '
    set -euo pipefail
    grep -q "formatIndexedVolumeUsd" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "charts-pair-volume-usd" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "formatChartsPairTokenVolume" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "volume_usd" frontend-dapp/src/types/index.ts
    grep -q "volume_usd" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -qE "formatNum\(stats\.volume_base" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -qE "formatNum\(stats\.volume_quote" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "volume_usd: Option<String>" indexer/src/api/pairs.rs
    grep -q "volume_base: stats.volume_base.to_string()" indexer/src/api/pairs.rs
    grep -q "volume_quote: stats.volume_quote.to_string()" indexer/src/api/pairs.rs
  '

run_step "frontend: pair token helper + ChartsPage 24h stats" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/chartsPairStats.test.ts \
    src/utils/__tests__/chartsOverviewStats.test.ts \
    src/pages/ChartsPage.test.tsx'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
