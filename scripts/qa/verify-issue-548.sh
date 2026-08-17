#!/usr/bin/env bash
# Verification for GitLab #548: Charts overview 24h volume USD-only + catalog volume_usd.
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
echo "  GitLab #548 — Charts overview USD-only + catalog volume_usd"
echo "════════════════════════════════════════════════════════════════"

run_step "docs: invariants + skill + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_CHARTS_OVERVIEW.md
    grep -q "Charts overview USD (#548)" docs/indexer-invariants.md
    grep -q "AGENTS_FRONTEND_CHARTS_OVERVIEW" AGENTS.md
    grep -q "charts-overview-volume-usd" docs/frontend.md
    grep -q "P522-Q catalog" docs/runbooks/indexer-external-oracle.md
    grep -q "volume_usd_for_swap" skills/AGENTS_INDEXER_PAIR_PRICE_USD.md
    grep -q "token_count" skills/AGENTS_INDEXER_VOLUME_PAGINATION.md
    grep -q "verify-issue-548" Makefile
    test -f indexer/migrations/20260817120000_backfill_swap_volume_usd_catalog.sql
  '

run_step "code: Charts does not format raw total_volume_24h" \
  bash -c '
    set -euo pipefail
    grep -q "formatChartsOverviewVolumeUsd" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "charts-overview-volume-usd" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -qE "formatNum\(overview\.total_volume_24h" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -q "lg:grid-cols-6" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "count_pair_leg_assets" indexer/src/api/overview.rs
    ! grep -q "get_all_assets" indexer/src/api/overview.rs
    grep -q "volume_usd_for_swap" indexer/src/indexer/parser.rs
    ! grep -q "decimals_factor = BigDecimal::from(1_000_000" indexer/src/indexer/parser.rs
  '

run_step "frontend: overview formatters + Charts strip" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/chartsOverviewStats.test.ts \
    src/pages/ChartsPage.test.tsx'

run_step "indexer lib: pair_price volume catalog + overview field" \
  bash -c 'cd indexer && cargo test --lib pair_price -- --quiet && cargo test --lib idle_dex_sends -- --quiet && cargo test --lib unpriced_activity -- --quiet && cargo test --lib priced_activity -- --quiet'

if [ -f "$REPO_ROOT/indexer/.env" ]; then
  export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
  run_step "indexer integration: catalog USD + overview token_count" \
    bash -c 'cd indexer && cargo test --test volume_usd_catalog --test api_overview --test indexer_overview_global_stats -- --test-threads=1 --quiet'
else
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
  export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
  run_step "indexer integration: catalog USD + overview token_count" \
    bash -c 'cd indexer && cargo test --test volume_usd_catalog --test api_overview --test indexer_overview_global_stats -- --test-threads=1 --quiet'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
