#!/usr/bin/env bash
# Automated verification for Forgejo #1227 — UpdateLimitOrderPrice joins
# the equal-price FIFO tail (keep order_id).
#
# Proves:
#   1. Pair unit: relink tail / same-price no-op / better-price head / hint.
#   2. Integration: bid+ask FIFO after reprice, price priority, atomic fail.
#   3. Existing new-place FIFO still green.
#   4. Docs / L23 / skill crosslinks.
# Optional (VERIFY1227_INDEXER=1, needs Postgres + indexer/.env):
#   resting_book_walk_index_preserves_reprice_fifo
#
# Refs: smartcontracts/contracts/pair/src/orderbook.rs,
#       docs/contracts-security-audit.md (L23),
#       skills/AGENTS_LIMIT_ORDER_REPRICE_FIFO.md
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
echo "  Forgejo #1227 — UpdateLimitOrderPrice equal-price FIFO"
echo "════════════════════════════════════════════════════════════════"

export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH:-}"

run_step "pair unit: fifo_after_update_limit_order_price" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair --lib fifo_after_update_limit_order_price -- --quiet'

run_step "pair unit: fifo_bid_after_update_limit_order_price_joins_equal_price_tail" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair --lib fifo_bid_after_update_limit_order_price_joins_equal_price_tail -- --quiet'

run_step "pair unit: fifo_ask_after_update_limit_order_price_joins_equal_price_tail" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair --lib fifo_ask_after_update_limit_order_price_joins_equal_price_tail -- --quiet'

run_step "integration: fifo after UpdateLimitOrderPrice (T1–T10 / A1–A4)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests after_update_limit_order_price -- --quiet'

run_step "integration: fifo_two_bids_same_price_older_filled_first (T3)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests fifo_two_bids_same_price_older_filled_first -- --quiet'

run_step "integration: fifo_two_asks_same_price_older_filled_first (AC3)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests fifo_two_asks_same_price_older_filled_first -- --quiet'

run_step "integration: update_limit_order_price_changes_price_not_remaining (#247)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests update_limit_order_price_changes_price_not_remaining -- --quiet'

run_step "docs: L23 + ordering + skill" \
  bash -c '
    set -euo pipefail
    rg -q "L23" docs/contracts-security-audit.md
    rg -q "RELINK_EQUAL_PRICE_SORT_ID" docs/contracts-security-audit.md
    rg -qi "time-priority at the quoted price" docs/limit-orders.md
    rg -q "joins the tail" docs/integrators.md
    rg -q "walk_index" docs/indexer-invariants.md
    test -f skills/AGENTS_LIMIT_ORDER_REPRICE_FIFO.md
    rg -q "R1227-1" skills/AGENTS_LIMIT_ORDER_REPRICE_FIFO.md
    rg -q "AGENTS_LIMIT_ORDER_REPRICE_FIFO" AGENTS.md
    rg -q "AGENTS_LIMIT_ORDER_REPRICE_FIFO" skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md
    rg -q "AGENTS_LIMIT_ORDER_REPRICE_FIFO" skills/AGENTS_BOOK_MATCH_HINT_SECURITY.md
    rg -q "AGENTS_LIMIT_ORDER_REPRICE_FIFO" skills/AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md
    rg -q "AGENTS_LIMIT_ORDER_REPRICE_FIFO" skills/AGENTS_INDEXER_AMM_ORDERBOOK_SIM.md
    rg -q "walk_index" indexer/src/db/queries/resting_orders.rs
    rg -q "RELINK_EQUAL_PRICE_SORT_ID" smartcontracts/contracts/pair/src/orderbook.rs
  '

if [[ "${VERIFY1227_INDEXER:-0}" == "1" || -f "$REPO_ROOT/indexer/.env" ]]; then
  run_step "indexer: resting_book_walk_index_preserves_reprice_fifo" \
    bash -c 'cd indexer && cargo test --test db_orderbook_mirror resting_book_walk_index_preserves_reprice_fifo -- --test-threads=1 --quiet'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> Forgejo #1227 verification passed"
