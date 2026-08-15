#!/usr/bin/env bash
# Verification for GitLab #522: pair Price (USD) human scale + oracle conversion.
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
echo "  GitLab #522 — pair Price (USD) scale + oracle"
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
    test -f skills/AGENTS_INDEXER_PAIR_PRICE_USD.md
    grep -q "Pair Price (USD) (#522)" docs/indexer-invariants.md
    grep -q "P522-1" docs/indexer-invariants.md
    grep -q "AGENTS_INDEXER_PAIR_PRICE_USD" AGENTS.md
    grep -q "resolveTapeLastPriceUsd" docs/frontend.md
    grep -q "price_usd" skills/AGENTS_FRONTEND_PRICE_CHART.md
    grep -q "#522" skills/AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md
    test -f indexer/migrations/20260815000000_swap_price_human_and_usd.sql
  '

run_step "indexer lib: pair_price_usd" \
  bash -c 'cd indexer && cargo test --lib pair_price -- --quiet'

run_step "indexer lib: swap_orientation" \
  bash -c 'cd indexer && cargo test --lib swap_orientation -- --quiet'

run_step "indexer integration: human USD + #466 orientation" \
  bash -c 'cd indexer && cargo test --test swap_price_human_usd --test swap_price_orientation -- --test-threads=1 --quiet'

run_step "frontend: tape USD helper + headline + formatPairPrice" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/utils/__tests__/pairPriceUsd.test.ts src/components/charts/__tests__/chartHeadlinePrice.test.ts src/components/ui/__tests__/TradesTable.test.tsx -t "resolveTape|pairStatsUsdField|formatPairPrice|Last price headline|18/6|renders trade rows"'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
