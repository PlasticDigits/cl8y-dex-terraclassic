#!/usr/bin/env bash
# Verification for GitLab #705: GET /candles newest-N + interval refit + chip contrast.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env; no wasm deploy).
# Marks still on GET: VERIFY_ISSUE_568_SKIP_RELATED=1 make verify-issue-568 unless skipped.
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
echo "  GitLab #705 — newest-N candles + interval refit + chip"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs: invariants + skills + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_INDEXER_CANDLES_NEWEST_N.md
    grep -q "Candle newest-N read (#705)" docs/indexer-invariants.md
    grep -q "C705-1" skills/AGENTS_INDEXER_CANDLES_NEWEST_N.md
    grep -q "C705-8" skills/AGENTS_INDEXER_CANDLES_NEWEST_N.md
    grep -q "AGENTS_INDEXER_CANDLES_NEWEST_N" AGENTS.md
    grep -q "verify-issue-705" AGENTS.md
    grep -q "verify-issue-705" docs/testing.md
    grep -q "#705" docs/frontend.md
    grep -q "price-chart-interval" docs/frontend.md
    grep -q "AGENTS_INDEXER_CANDLES_NEWEST_N" skills/AGENTS_FRONTEND_PRICE_CHART.md
    grep -q "AGENTS_INDEXER_CANDLES_NEWEST_N" skills/AGENTS_INDEXER_CANDLE_USD_MARK.md
    grep -q "price-chart-interval" skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md
  '

run_step "source: newest-N SQL; interval fitContent; chart-interval chip; no *-neo" \
  bash -c '
    set -euo pipefail
    grep -q "ORDER BY open_time DESC" indexer/src/db/queries/candles.rs
    grep -q "ORDER BY open_time ASC" indexer/src/db/queries/candles.rs
    grep -q "fittedIntervalRef" frontend-dapp/src/components/charts/PriceChartLightweightCanvas.tsx
    grep -q "price-chart-interval" frontend-dapp/src/components/charts/PriceChart.tsx
    grep -q "price-chart-interval.tab-glass-active" frontend-dapp/src/index.css
    grep -q "interval={interval}" frontend-dapp/src/components/charts/PriceChart.tsx
    if grep -nE "price-chart-interval.*-neo|-neo.*price-chart-interval" frontend-dapp/src/index.css frontend-dapp/src/components/charts/PriceChart.tsx; then
      echo "chart interval must not use *-neo" >&2
      exit 1
    fi
    if grep -nE "\.price-chart-interval\.tab-glass-active" -A 12 frontend-dapp/src/index.css | grep -q "0\.14"; then
      echo "chart-interval active must not be the 14% wash" >&2
      exit 1
    fi
  '

run_step "indexer integration: newest-N + limit clamp" \
  bash -c 'cd indexer && cargo test --test api_candles_newest_n -- --test-threads=1 --quiet && cargo test --test api_limit_lower_bound pair_candles -- --test-threads=1 --quiet'

run_step "frontend: PriceChart interval + canvas fitContent split + candles + chip CSS" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/components/charts/__tests__/PriceChart.test.tsx \
    src/components/charts/__tests__/PriceChartLightweightCanvas.test.tsx \
    src/components/charts/__tests__/priceChartCandles.test.ts \
    src/designTokens.test.ts'

if [[ "${VERIFY_ISSUE_705_SKIP_568:-}" == "1" ]]; then
  echo ""
  echo "[related #568] skipped (VERIFY_ISSUE_705_SKIP_568=1)"
  ok "related candle mark GET (skipped)"
else
  run_step "related: mark bars still last on GET (#568 / C705-7)" \
    bash -c 'cd indexer && cargo test --test candle_usd_mark get_candles_includes_mark_bars -- --test-threads=1 --quiet'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
