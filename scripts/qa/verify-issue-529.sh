#!/usr/bin/env bash
# Automated verification for GitLab #529 — decimals-normalized limit price band.
#
# Proves:
#   1. human_scale / validate_limit_order_price accepts 6-vs-18 raw ~7.9e13.
#   2. Equal-decimal dust still rejected (#467 not reopened).
#   3. Integration: place + fill on 6/18; place on 18/6.
#   4. Frontend human↔raw helpers + submit-edge scale.
#   5. Docs / skill / L20 crosslinks.
#
# Refs: smartcontracts/packages/dex-common/src/limit_placement.rs,
#       docs/contracts-security-audit.md (L20), skills/AGENTS_LIMIT_PRICE_DECIMALS.md.
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
echo "  GitLab #529 — limit price human-scale band (6-vs-18 pairs)"
echo "════════════════════════════════════════════════════════════════"

run_step "dex-common validate + 6/18 unit tests" \
  bash -c 'cd smartcontracts && cargo test -p dex-common limit_placement::tests::validate_limit_price --quiet && cargo test -p dex-common expand_ladder_accepts_six_vs_eighteen --quiet'

run_step "integration: place_and_fill_limit_on_six_vs_eighteen_pair" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests place_and_fill_limit_on_six_vs_eighteen_pair --quiet'

run_step "integration: place_limit_on_eighteen_vs_six_pair" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests place_limit_on_eighteen_vs_six_pair --quiet'

run_step "integration: place_limit_order_dust_price_rejected (#467)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests place_limit_order_dust_price_rejected --quiet'

run_step "frontend: limitOrderPriceScale + pair submit scale" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run limitOrderPriceScale pair.test

run_step "docs: L20 mentions #529 human-scale" \
  grep -q 'decimals-normalized \[\#529\]' docs/contracts-security-audit.md

run_step "docs: limit-orders #529 band" \
  grep -q 'decimals0 − decimals1' docs/limit-orders.md

run_step "skill: AGENTS_LIMIT_PRICE_DECIMALS L529" \
  grep -q 'L529-1' skills/AGENTS_LIMIT_PRICE_DECIMALS.md

run_step "skill: BOOK_MATCH_HINT L20 mentions #529" \
  grep -q 'decimals-normalized #529' skills/AGENTS_BOOK_MATCH_HINT_SECURITY.md

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #529 verification passed"
