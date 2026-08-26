#!/usr/bin/env bash
# Verification for GitLab #553: Charts leaderboard + trader profile volume USD.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env; no wasm deploy) for indexer steps.
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
echo "  GitLab #553 — trader leaderboard + profile volume USD"
echo "════════════════════════════════════════════════════════════════"

run_step "docs: invariants + skill + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_TRADER_VOLUME_USD.md
    grep -q "Trader volume USD (#553)" docs/indexer-invariants.md
    grep -q "AGENTS_FRONTEND_TRADER_VOLUME_USD" AGENTS.md
    grep -q "charts-leaderboard-volume" docs/frontend.md
    grep -q "T553-5" skills/AGENTS_FRONTEND_TRADER_VOLUME_USD.md
    grep -q "verify-issue-553" Makefile
    grep -q "#553" skills/AGENTS_FRONTEND_CHARTS_OVERVIEW.md
    grep -q "total_volume_usd" skills/AGENTS_FRONTEND_PORTFOLIO.md
    test -f indexer/migrations/20260818140000_traders_total_volume_usd.sql
  '

run_step "code: UI does not formatNum raw total_volume" \
  bash -c '
    set -euo pipefail
    grep -q "formatIndexedVolumeUsd" frontend-dapp/src/components/trader/TraderLeaderboard.tsx
    grep -q "charts-leaderboard-volume" frontend-dapp/src/components/trader/TraderLeaderboard.tsx
    grep -q "total_volume_usd" frontend-dapp/src/components/trader/TraderLeaderboard.tsx
    grep -q "TraderLeaderboard" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -qE "formatNum\(.*total_volume[^_]" frontend-dapp/src/components/trader/TraderLeaderboard.tsx
    ! grep -qE "formatNum\(.*total_volume[^_]" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -q "formatNum(trader.total_volume)" frontend-dapp/src/components/trader/TraderSummaryStats.tsx
    grep -q "trader-total-volume-usd" frontend-dapp/src/components/trader/TraderSummaryStats.tsx
    grep -q "volume_usd.as_ref()" indexer/src/indexer/parser.rs
    grep -q "refresh_trader_total_volume_usd" indexer/src/db/queries/traders.rs
    grep -q "total_volume_usd" indexer/src/api/traders.rs
  '

run_step "frontend: USD formatters + leaderboard + profile stats" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/chartsOverviewStats.test.ts \
    src/pages/ChartsPage.test.tsx \
    src/components/trader/TraderLeaderboard.test.tsx \
    src/components/trader/TraderSummaryStats.test.tsx \
    src/services/indexer/traderProfilePayload.test.ts'

if [ -f "$REPO_ROOT/indexer/.env" ]; then
  export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
  run_step "indexer integration: trader USD + leaderboard sort" \
    bash -c 'cd indexer && cargo test --test trader_volume_usd -- --test-threads=1 --quiet && cargo test --test api_traders -- --test-threads=1 --quiet'
else
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
  export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
  run_step "indexer integration: trader USD + leaderboard sort" \
    bash -c 'cd indexer && cargo test --test trader_volume_usd -- --test-threads=1 --quiet && cargo test --test api_traders -- --test-threads=1 --quiet'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
