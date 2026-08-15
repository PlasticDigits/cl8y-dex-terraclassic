#!/usr/bin/env bash
# Automated verification for GitLab #524 — /trade + /charts UST1 pair display invert.
#
# Proves (unit + docs; no chain required):
#   1. Default invert helper, USD/OHLC invert, side + price convert (H1).
#   2. PriceChart pill + aria-live names the displayed token.
#   3. TradePage UST1 default, pill/ticket sync, no Order ticket, pair-switch isolation.
#   4. Docs/skills/invariants T524-1–T524-11 crosslinked; AGENTS playbook present.
#   5. Indexer #522 helpers still prefer price_usd (no raw-price-as-USD regression).
#
# Refs: skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md,
#       frontend-dapp/src/utils/tradePairDisplayOrientation.ts,
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
echo "  GitLab #524 — Trade / Charts UST1 pair display invert"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: invert helper + USD + candles + trade page" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/tradePairDisplayOrientation.test.ts \
    src/utils/__tests__/pairPriceUsd.test.ts \
    src/components/charts/__tests__/priceChartCandles.test.ts \
    src/components/trade/__tests__/TradeOrderTicket.invert.test.tsx \
    src/utils/__tests__/pairMenuOptions.test.ts \
    src/pages/TradePage.test.tsx'

run_step "frontend: PriceChart pill + XSS text nodes (#524)" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/components/charts/__tests__/PriceChart.test.tsx -t "524|H7"'

run_step "code: convert-on-submit uses factorySide + factoryPrice" \
  grep -qE 'factorySideFromDisplay' frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
  grep -qE 'displayPriceToFactoryToken1PerToken0' frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
  grep -qE 'placeLimitOrderWithAllowance\(' frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
  grep -qE 'factorySide,' frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
  grep -qE 'factoryPrice' frontend-dapp/src/components/trade/TradeOrderTicket.tsx

run_step "code: no Order ticket eyebrow" \
  bash -c '! grep -qE "Order ticket|ORDER TICKET" frontend-dapp/src/components/trade/TradeOrderTicket.tsx'

run_step "code: pair invert is not LimitOrderSideFlipButton" \
  grep -qE 'trade-ticket-pair-invert' frontend-dapp/src/components/trade/PairDisplayInvertControls.tsx && \
  grep -qE 'limit-order-side-flip' frontend-dapp/src/components/trade/LimitOrderPriceField.tsx

run_step "docs: frontend.md T524-1–T524-11" \
  grep -qE 'trade-pair-display-invert' docs/frontend.md && \
  grep -qE '\*\*T524-1\*\*' docs/frontend.md && \
  grep -qE '\*\*T524-11\*\*' docs/frontend.md

run_step "skill: AGENTS_FRONTEND_TRADE_PAIR_INVERT" \
  grep -qE '\*\*T524-1' skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md && \
  grep -qE 'displayPriceToFactoryToken1PerToken0' skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md && \
  grep -qE 'make verify-issue-524' skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md

run_step "skill: price chart + trade layout + pair USD crosslinks #524" \
  grep -qE 'AGENTS_FRONTEND_TRADE_PAIR_INVERT|#524' skills/AGENTS_FRONTEND_PRICE_CHART.md && \
  grep -qE 'AGENTS_FRONTEND_TRADE_PAIR_INVERT|#524' skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md && \
  grep -qE 'AGENTS_FRONTEND_TRADE_PAIR_INVERT|#524' skills/AGENTS_INDEXER_PAIR_PRICE_USD.md

run_step "AGENTS.md playbook link #524" \
  grep -qE 'AGENTS_FRONTEND_TRADE_PAIR_INVERT|#524' AGENTS.md

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
