#!/usr/bin/env bash
# Automated verification for GitLab #543 — Price (USD) candles match Last (invertUsd, not 1/x).
#
# Proves (unit + docs; indexer integration when Postgres is available):
#   1. invertUsd vs invertOhlc fixtures (USTR 0.012…; cLUNC invert → ~$1 not 21260).
#   2. applyChartDisplayInvert uses factory USD / human; drops missing USD / ≤0 / non-finite.
#   3. Adaptive usdCandlePriceFormat distinguishes 1.06 / 0.012258 / 0.000047.
#   4. PriceChart inverted last-close headline is invertUsd; canvas applyOptions priceFormat.
#   5. Docs/skills C543 + #524 playbook states USD candles use invertUsd.
#   6. Indexer rebuild writes USD from price_usd only + additive *_human (if indexer/.env).
#
# Refs: skills/AGENTS_FRONTEND_USD_CANDLE_INVERT.md,
#       frontend-dapp/src/components/charts/priceChartCandles.ts,
#       docs/frontend.md § Trade pair display invert
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
echo "  GitLab #543 — USD candles invertUsd + adaptive axis"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: invertUsd vs invertOhlc + factory candles + format" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/tradePairDisplayOrientation.test.ts \
    src/utils/__tests__/pairPriceUsd.test.ts \
    src/components/charts/__tests__/priceChartCandles.test.ts \
    src/components/charts/__tests__/priceChartPriceScale.test.ts \
    src/components/charts/__tests__/priceChartLightweightSeriesSync.test.ts \
    src/components/charts/__tests__/PriceChart.test.tsx \
    src/components/charts/__tests__/PriceChartLightweightCanvas.test.tsx'

run_step "code: applyChartDisplayInvert uses invertUsdNumber not invertOhlc" \
  bash -c 'grep -qE "invertUsdNumber" frontend-dapp/src/components/charts/priceChartCandles.ts &&
    grep -qE "indexerCandlesToFactoryPoints" frontend-dapp/src/components/charts/PriceChart.tsx &&
    ! grep -qE "import \\{ invertOhlc" frontend-dapp/src/components/charts/priceChartCandles.ts'

run_step "code: adaptive USD priceFormat via applyOptions" \
  bash -c 'grep -qE "usdCandlePriceFormatFromPoints" frontend-dapp/src/components/charts/PriceChartLightweightCanvas.tsx &&
    grep -qE "priceFormat: usdCandlePriceFormatFromPoints" frontend-dapp/src/components/charts/PriceChartLightweightCanvas.tsx &&
    grep -qE "applyOptions" frontend-dapp/src/components/charts/PriceChartLightweightCanvas.tsx'

run_step "code: indexer USD candles omit human fallback" \
  bash -c 'grep -qE "open_human" indexer/src/db/queries/candles.rs &&
    grep -qE "se.price_usd IS NOT NULL" indexer/src/db/queries/candles.rs &&
    ! grep -qE "COALESCE\\(se.price_usd, se.price\\)" indexer/src/db/queries/candles.rs &&
    grep -qE "price_usd.as_ref\\(\\)" indexer/src/indexer/parser.rs &&
    ! grep -qE "unwrap_or\\(&oriented.price\\)" indexer/src/indexer/parser.rs'

run_step "docs: frontend.md C543 + invertUsd vs invertOhlc" \
  bash -c 'grep -qE "\\*\\*C543-1\\*\\*" docs/frontend.md &&
    grep -qE "\\*\\*C543-9\\*\\*" docs/frontend.md &&
    grep -qE "invertUsd" docs/frontend.md &&
    grep -qE "Never.*1 / price_usd" docs/frontend.md'

run_step "skill: AGENTS_FRONTEND_USD_CANDLE_INVERT + crosslinks" \
  bash -c 'test -f skills/AGENTS_FRONTEND_USD_CANDLE_INVERT.md &&
    grep -qE "invertUsd" skills/AGENTS_FRONTEND_USD_CANDLE_INVERT.md &&
    grep -qE "make verify-issue-543" skills/AGENTS_FRONTEND_USD_CANDLE_INVERT.md &&
    grep -qE "AGENTS_FRONTEND_USD_CANDLE_INVERT|#543" skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md &&
    grep -qE "AGENTS_FRONTEND_USD_CANDLE_INVERT|#543" skills/AGENTS_FRONTEND_PRICE_CHART.md &&
    grep -qE "AGENTS_FRONTEND_USD_CANDLE_INVERT|#543" skills/AGENTS_INDEXER_PAIR_PRICE_USD.md &&
    grep -qE "AGENTS_FRONTEND_USD_CANDLE_INVERT|#543" AGENTS.md'

run_step "docs: indexer-invariants additive human OHLC" \
  bash -c 'grep -qE "open_human" docs/indexer-invariants.md &&
    grep -qE "#543" docs/indexer-invariants.md'

if [ -f "$REPO_ROOT/indexer/.env" ]; then
  export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
  run_step "indexer: candle human+USD rebuild + skip missing USD" \
    bash -c 'cd indexer && cargo test --test candle_human_usd --test candle_skip_zero_price -- --test-threads=1 --quiet'
else
  echo ""
  echo "[indexer integration] indexer/.env missing — skip (run make setup-indexer-postgres)"
  ok "indexer integration skipped (no indexer/.env)"
fi

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
