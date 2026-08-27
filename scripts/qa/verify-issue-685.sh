#!/usr/bin/env bash
# Automated verification for GitLab #685 — CG/CMC listing field truthfulness.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env).
#
# Refs: skills/AGENTS_INDEXER_CG_CMC_LISTING.md,
#       docs/CG_CMC_COMPLIANCE.md,
#       docs/indexer-invariants.md
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

docs_crosslinks() {
  grep -qE "L685-1" skills/AGENTS_INDEXER_CG_CMC_LISTING.md
  grep -qE "L685-8" skills/AGENTS_INDEXER_CG_CMC_LISTING.md
  grep -qE "make verify-issue-685" skills/AGENTS_INDEXER_CG_CMC_LISTING.md
  grep -qE "CG/CMC listing field truthfulness" docs/indexer-invariants.md
  grep -qE "pair_liquidity_usd" docs/CG_CMC_COMPLIANCE.md
  grep -qE "offers? quote" docs/CG_CMC_COMPLIANCE.md
  grep -qE "max \*\*500\*\*" docs/CG_CMC_COMPLIANCE.md
  grep -qE "AGENTS_INDEXER_CG_CMC_LISTING" AGENTS.md
  grep -qE "verify-issue-685" AGENTS.md
  grep -qE "verify-issue-685" docs/testing.md
  grep -qE "#685" docs/listings/forms/coingecko-exchange.md
  grep -qE "AGENTS_INDEXER_CG_CMC_LISTING" skills/AGENTS_LISTINGS.md
  if grep -nE "mislabeled 24h volume|last_price \* 0\.999" docs/CG_CMC_COMPLIANCE.md \
       docs/indexer-invariants.md docs/runbooks/overview-global-stats-brin.md \
       skills/AGENTS_DEFILLAMA.md 2>/dev/null; then
    echo "listing docs still call liquidity_in_usd mislabeled volume or 0.999 spread" >&2
    return 1
  fi
}

source_guards() {
  grep -qE "listing_liquidity_in_usd" indexer/src/api/cg.rs
  grep -qE "listing_bid_ask" indexer/src/api/cg.rs
  grep -qE "listing_bid_ask" indexer/src/api/cmc.rs
  grep -qE "is_pair_code_id_frozen" indexer/src/api/cmc.rs
  grep -qE "listing_cmc_unified_id" indexer/src/api/cmc.rs
  grep -qE "pair_is_listing_excluded" indexer/src/api/aggregator_snapshot.rs
  grep -qE "get_all_pair_liquidity_usd" indexer/src/api/aggregator_snapshot.rs
  grep -qE "record_unique_ticker" indexer/src/api/mod.rs
  grep -qE "pub fn is_excluded_cw20" indexer/src/indexer/listing_exclude.rs
  if grep -nE "last_price_f \* 0\.999|to_f64\(\).*0\.999" indexer/src/api/cg.rs indexer/src/api/cmc.rs 2>/dev/null; then
    echo "toy f64 * 0.999 bid/ask must not remain" >&2
    return 1
  fi
  if grep -nE "volume_usd" indexer/src/api/cg.rs | grep -q liquidity; then
    echo "cg.rs must not copy volume_usd into liquidity_in_usd" >&2
    return 1
  fi
}

indexer_lib() {
  (cd indexer && cargo test --lib listing_exclude -- --quiet && cargo test --lib listing_spread -- --quiet)
}

indexer_integration() {
  (cd indexer && cargo test --test api_cg_cmc_listing --test api_cg --test api_cmc --test api_aggregator_batch --test api_gt -- --test-threads=1 --quiet)
}

echo "================================================================"
echo "  GitLab #685 — CG/CMC listing field truthfulness"
echo "================================================================"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs: invariants + skills + AGENTS crosslinks" docs_crosslinks
run_step "source: TVL stamp, no f64 toy spread, shared gem helper" source_guards
run_step "indexer lib: listing_exclude + listing_spread" indexer_lib
run_step "indexer integration: cg/cmc listing + gt lockstep" indexer_integration

echo ""
echo "================================================================"
echo "  Results: ${PASS} passed, ${FAIL} failed"
echo "================================================================"
for line in "${RESULTS[@]}"; do
  echo "  $line"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
