#!/usr/bin/env bash
# Automated verification for #1315 — ALPHA pair price candles.
#
# Proves (unit + docs; no chain, no Postgres, no Playwright):
#   1. candle_usd_subject slot identity, no ALPHA/CL8Y peg, skip rules.
#   2. GET projection omits open/high/low/close when usd_leg=asset_1.
#   3. Gap-fill done marker only when the failure count is 0.
#   4. down.sql deletes stamped keys, usd_leg=asset_1, and NULL open.
#   5. priceChartCandles + PriceChart human-only fixture + Charts pill default.
#
# Refs: docs/adr/0012-alpha-pair-price-candles.md,
#       skills/AGENTS_INDEXER_PAIR_PRICE_USD.md
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
echo "  #1315 — ALPHA pair price candles"
echo "════════════════════════════════════════════════════════════════"

DOWN="indexer/migrations/20260922120000_candle_usd_leg_1315.down.sql"
run_step "down.sql deletes gap keys, asset_1 rows, and NULL open" \
  grep -q "usd_leg = 'asset_1'" "$DOWN"
run_step "down.sql deletes NULL open" \
  grep -q "open IS NULL" "$DOWN"
run_step "down.sql deletes stamped gap-fill keys" \
  grep -q "candle_gap_fill_1315" "$DOWN"

if (
  cd indexer
  cargo test --lib candle_subject -- --test-threads=1
  cargo test --lib project_tests -- --test-threads=1
  cargo test --lib gap_fill -- --test-threads=1
); then
  ok "indexer lib candle subject, wire projection, gap-fill marker"
else
  bad "indexer lib candle subject, wire projection, gap-fill marker"
fi

run_step "priceChartCandles + PriceChart + Charts orientation" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/components/charts/__tests__/priceChartCandles.test.ts \
    src/components/charts/__tests__/PriceChart.test.tsx \
    src/utils/__tests__/tradePairDisplayOrientation.test.ts

echo ""
echo "════════════════════════════════════════════════════════════════"
if [[ "$FAIL" -eq 0 ]]; then
  echo "  #1315 verify: ${PASS} passed"
  echo "════════════════════════════════════════════════════════════════"
  exit 0
fi
echo "  #1315 verify: ${PASS} passed, ${FAIL} failed" >&2
printf '  %s\n' "${RESULTS[@]}" >&2
echo "════════════════════════════════════════════════════════════════" >&2
exit 1
