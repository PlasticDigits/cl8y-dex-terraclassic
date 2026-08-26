#!/usr/bin/env bash
# Verification for GitLab #666: /charts pair-scoped 24h stats + leaderboard
# (drop DEX-wide overview census).
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env; no wasm deploy)
# for indexer steps. Playwright smoke is optional (PLAYWRIGHT_SKIP_CHAIN).
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
echo "  GitLab #666 — Charts pair-scoped 24h stats + leaderboard"
echo "════════════════════════════════════════════════════════════════"

run_step "docs: invariants + skill + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md
    grep -q "Charts pair-scoped stats + leaderboard (#666)" docs/indexer-invariants.md
    grep -q "AGENTS_FRONTEND_CHARTS_PAIR_SCOPED" AGENTS.md
    grep -q "verify-issue-666" AGENTS.md
    grep -q "charts-pair-scoped" docs/frontend.md
    grep -q "CS-1" skills/AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md
    grep -q "CS-15" skills/AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md
    grep -q "verify-issue-666" Makefile
    grep -q "#666" skills/AGENTS_FRONTEND_CHARTS_OVERVIEW.md
    grep -q "#666" skills/AGENTS_FRONTEND_CHARTS_PAIR_STATS.md
    grep -q "#666" skills/AGENTS_FRONTEND_TRADER_VOLUME_USD.md
    grep -q "#666" docs/testing.md
  '

run_step "code: Charts has no census strip; pair= on leaderboard" \
  bash -c '
    set -euo pipefail
    ! grep -q "charts-overview-pairs" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -q "charts-overview-tokens" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -q "charts-overview-volume-usd" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -q "getOverview" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "charts-pair-24h-stats" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "getLeaderboard(leaderboardSort, 20, activePairAddr)" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "No traders on this pair yet" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -q "Best Trade" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "PAIR_SCOPED_SORTS" indexer/src/api/traders.rs
    grep -q "get_leaderboard_for_pair" indexer/src/db/queries/traders.rs
    grep -q "pair_or_-" indexer/src/api/traders.rs || grep -q "{sort}|{limit}|{pair" indexer/src/api/traders.rs
    grep -q "variant=\"flat\"" frontend-dapp/src/pages/ChartsPage.tsx
  '

run_step "frontend: pair-scoped ChartsPage + getLeaderboard query" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/ChartsPage.test.tsx \
    src/services/indexer/__tests__/client.test.ts'

run_step "guard: check_chrome_nesting.py" \
  python3 scripts/check_chrome_nesting.py

if [ -f "$REPO_ROOT/indexer/.env" ]; then
  export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
  run_step "indexer integration: pair leaderboard + injection" \
    bash -c 'cd indexer && cargo test --test api_traders --test security -- --test-threads=1 --quiet'
else
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
  export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
  run_step "indexer integration: pair leaderboard + injection" \
    bash -c 'cd indexer && cargo test --test api_traders --test security -- --test-threads=1 --quiet'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
echo "==> GitLab #666 verification passed"
