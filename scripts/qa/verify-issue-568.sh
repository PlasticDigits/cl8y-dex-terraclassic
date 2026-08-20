#!/usr/bin/env bash
# Verification for GitLab #568: time-stamped candle USD + idle mark-to-market.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env; no wasm deploy).
# Related ladders (556/543/522/515) run unless VERIFY_ISSUE_568_SKIP_RELATED=1.
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
echo "  GitLab #568 — time-stamped candle USD + idle mark-to-market"
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
    test -f skills/AGENTS_INDEXER_CANDLE_USD_MARK.md
    grep -q "Candle USD clock (#568)" docs/indexer-invariants.md
    grep -q "C568-1" skills/AGENTS_INDEXER_CANDLE_USD_MARK.md
    grep -q "C568-8" skills/AGENTS_INDEXER_CANDLE_USD_MARK.md
    grep -q "AGENTS_INDEXER_CANDLE_USD_MARK" AGENTS.md
    grep -q "verify-issue-568" AGENTS.md
    grep -q "verify-issue-568" docs/testing.md
    grep -q "#568" docs/frontend.md
    grep -q "C568-1" skills/AGENTS_INDEXER_HUB_USD.md
    grep -q "AGENTS_INDEXER_CANDLE_USD_MARK" skills/AGENTS_INDEXER_PAIR_PRICE_USD.md
    grep -q "AGENTS_INDEXER_CANDLE_USD_MARK" skills/AGENTS_FRONTEND_USD_CANDLE_INVERT.md
    test -f indexer/migrations/20260820156800_repair_candle_usd_as_of_oracle.sql
  '

run_step "source: no as-of-now hub rewrite; marks are current-bucket only" \
  bash -c '
    set -euo pipefail
    if grep -nF "backfill_usd_from_hub" indexer/src/db/queries/hub_prices.rs indexer/src/indexer/*.rs; then
      echo "backfill_usd_from_hub must not remain on the refresh path" >&2
      exit 1
    fi
    grep -q "apply_idle_usd_marks" indexer/src/db/queries/hub_prices.rs
    grep -q "update_candles_for_mark" indexer/src/indexer/candle_builder.rs
    grep -q "trade_count" indexer/src/indexer/candle_builder.rs
    grep -q "repair_ustc_lunc_usd_as_of_oracle" indexer/src/db/queries/usd_as_of.rs
    grep -q "UST1/USTR-quoted history cannot" indexer/migrations/20260820156800_repair_candle_usd_as_of_oracle.sql
    grep -qF "ALL: [OracleTicker; 3]" indexer/src/indexer/oracle.rs
    ! grep -q "OracleTicker::Ustr" indexer/src/indexer/oracle.rs
    if grep -nE "coingecko|oracle/history" frontend-dapp/src/components/charts/PriceChart.tsx frontend-dapp/src/components/charts/priceChartCandles.ts; then
      echo "Price (USD) charts must not stitch CoinGecko or /oracle/history" >&2
      exit 1
    fi
    grep -q "invertUsdNumber" frontend-dapp/src/components/charts/priceChartCandles.ts
  '

run_step "indexer lib: mark_price_usd + mark_quote_kind" \
  bash -c 'cd indexer && cargo test --lib --quiet pair_price && cargo test --lib --quiet candle_mark'

run_step "indexer integration: no rewrite + idle marks + as-of repair" \
  bash -c 'cd indexer && cargo test --test candle_usd_mark --test api_hub_prices --test candle_human_usd --test candle_skip_zero_price -- --test-threads=1 --quiet'

run_step "frontend: per-bar invertUsd is not flattened by as-of quote USD" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/components/charts/__tests__/priceChartCandles.test.ts src/utils/__tests__/tradePairDisplayOrientation.test.ts'

if [[ "${VERIFY_ISSUE_568_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related 556/543/522/515] skipped (VERIFY_ISSUE_568_SKIP_RELATED=1)"
  ok "related verifies (skipped)"
else
  run_step "related: verify-issue-556" \
    bash -c 'VERIFY_ISSUE_556_SKIP_RELATED=1 make verify-issue-556'
  run_step "related: verify-issue-543" make verify-issue-543
  run_step "related: verify-issue-522" make verify-issue-522
  run_step "related: verify-issue-515" make verify-issue-515
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
