#!/usr/bin/env bash
# Verification for GitLab #557: human Amount in / Amount out / Price on tape + wallet history.
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
echo "  GitLab #557 — human tape / wallet amounts"
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
    test -f skills/AGENTS_FRONTEND_TAPE_AMOUNTS.md
    grep -q "Trade tape decimals (#557)" docs/indexer-invariants.md
    grep -q "T557-1" docs/frontend.md
    grep -q "T557-11" skills/AGENTS_FRONTEND_TAPE_AMOUNTS.md
    grep -q "AGENTS_FRONTEND_TAPE_AMOUNTS" AGENTS.md
    grep -q "offer_decimals" docs/frontend.md
    grep -q "#557" skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md
    grep -q "#557" skills/AGENTS_INDEXER_PAIR_PRICE_USD.md
    grep -q "formatTapeAmount" skills/AGENTS_FRONTEND_ORDER_HISTORY.md
    grep -q "bd_plain_string" indexer/src/api/pairs.rs
    grep -q "to_plain_string" indexer/src/api/pairs.rs
  '

run_step "grep: no formatNum on tape raw amounts" \
  bash -c '
    set -euo pipefail
    if grep -nE "formatNum\(t\.(offer_amount|return_amount)\)" frontend-dapp/src/components/ui/TradesTable.tsx; then
      echo "TradesTable still formatNum(raw amounts)" >&2
      exit 1
    fi
    if grep -nE "formatNum\(raw\)|formatHistoryAmount" frontend-dapp/src/components/trade/WalletIndexerHistoryPanel.tsx; then
      echo "WalletIndexerHistoryPanel still formatNum(raw)" >&2
      exit 1
    fi
    grep -q "formatTapeAmount" frontend-dapp/src/components/ui/TradesTable.tsx
    grep -q "inverted" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "inverted={pairOrientation.inverted}" frontend-dapp/src/pages/TradePage.tsx
  '

run_step "indexer lib: tape decimals clamp" \
  bash -c 'cd indexer && cargo test --lib tape_decimals -- --quiet'

run_step "indexer integration: trade JSON + CSV decimals" \
  bash -c 'cd indexer && cargo test --test api_trade_decimals -- --test-threads=1 --quiet'

run_step "frontend: tape helpers + TradesTable + wallet history" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/utils/__tests__/tradeTapeDisplay.test.ts src/components/ui/__tests__/TradesTable.test.tsx src/components/trade/__tests__/WalletIndexerHistoryPanel.test.tsx src/components/trade/__tests__/TradeRecentTradesSection.test.tsx src/utils/__tests__/formatAmount.test.ts'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
