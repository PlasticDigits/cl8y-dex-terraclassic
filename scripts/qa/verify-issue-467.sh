#!/usr/bin/env bash
# Automated verification for GitLab #467 — limit order price band (dust ask DoS).
#
# Proves:
#   1. validate_limit_order_price rejects Decimal::raw(1) and accepts in-band prices.
#   2. Placement rejects dust ask/bid before the order reaches the book.
#   3. Crossing hybrid swap still fills a valid ask when dust placement is blocked.
#   4. Legacy out-of-band resting ask at book head is skipped (not a whole-swap revert).
#
# Refs: smartcontracts/packages/dex-common/src/limit_placement.rs,
#       smartcontracts/contracts/pair/src/orderbook.rs,
#       docs/contracts-security-audit.md (L19), docs/limit-orders.md.
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
echo "  GitLab #467 — limit order price band (dust ask match DoS)"
echo "════════════════════════════════════════════════════════════════"

run_step "dex-common validate_limit_order_price unit tests" \
  bash -c 'cd smartcontracts && cargo test -p dex-common limit_placement::tests::validate_limit_price --quiet && cargo test -p dex-common expand_ladder_rejects_out_of_band --quiet'

run_step "integration: place_limit_order_dust_price_rejected" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests place_limit_order_dust_price_rejected --quiet'

run_step "integration: dust_ask_brick_attack_prevented_valid_ask_still_fills" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests dust_ask_brick_attack_prevented_valid_ask_still_fills --quiet'

run_step "pair unit: match_asks_skips_legacy_dust_price_without_reverting" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair match_asks_skips_legacy_dust_price_without_reverting --quiet'

run_step "docs: L19 invariant row present" \
  grep -q 'L19.*Limit price band' docs/contracts-security-audit.md

run_step "docs: limit-orders price band section" \
  grep -q 'limit-price-band-gitlab-467' docs/limit-orders.md

run_step "skill: AGENTS_BOOK_MATCH_HINT_SECURITY L19" \
  grep -q 'L19 / GitLab #467' skills/AGENTS_BOOK_MATCH_HINT_SECURITY.md

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #467 verification passed"
