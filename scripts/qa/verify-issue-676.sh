#!/usr/bin/env bash
# Verification for GitLab #676: /positions trade_count on 18-decimal pairs.
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
echo "  GitLab #676 — trader positions 18-decimal storage"
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
    test -f skills/AGENTS_INDEXER_TRADER_POSITIONS_DECIMALS.md
    grep -q "Trader positions 18-decimal storage (#676)" docs/indexer-invariants.md
    grep -q "P676-1" docs/indexer-invariants.md
    grep -q "P676-8" skills/AGENTS_INDEXER_TRADER_POSITIONS_DECIMALS.md
    grep -q "AGENTS_INDEXER_TRADER_POSITIONS_DECIMALS" AGENTS.md
    grep -q "verify-issue-676" AGENTS.md
    grep -q "P676-1" docs/frontend.md
    grep -q "NUMERIC(78, 18)" docs/runbooks/indexer-reorg-replay-dedup.md
    grep -q "rebuild-positions" docs/runbooks/indexer-reorg-replay-dedup.md
    grep -q "P676" skills/AGENTS_FRONTEND_PORTFOLIO.md
    grep -q "P676" skills/AGENTS_FRONTEND_PORTFOLIO_PNL.md
  '

run_step "indexer lib: position_tracker human avg + clamp" \
  bash -c 'cd indexer && cargo test --lib position_tracker -- --quiet'

run_step "indexer integration: 18-dec persist + trade_count + rebuild" \
  bash -c 'cd indexer && cargo test --test position_tracker_18dec -- --test-threads=1 --quiet'

run_step "indexer integration: 6/6 clamp still green" \
  bash -c 'cd indexer && cargo test --test position_tracker_clamp -- --test-threads=1 --quiet'

run_step "indexer integration: positions API still exposes decimals" \
  bash -c 'cd indexer && cargo test --test api_traders get_trader_positions_returns_rows -- --test-threads=1 --quiet'

run_step "frontend: 18-dec raw 1e20 human scale" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/utils/__tests__/traderPositionDisplay.test.ts'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
