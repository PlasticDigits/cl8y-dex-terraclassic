#!/usr/bin/env bash
# Automated verification for GitLab #615 — tax-aware route/solve ranking.
#
# Proves:
#   1. Historical R615 legacy rules + current route-local v1 override in skills/docs
#   2. Independent trading discovery + funded ranking/cache/boundary lib tests
#   3. Frontend You Receive net helpers
#   4. #607 legacy direct compatibility + authenticated route-local security still green
#
# Refs: skills/AGENTS_INDEXER_TAX_AWARE_ROUTING.md
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
echo "  GitLab #615 — tax-aware route/solve ranking"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
if [[ -e "$REPO_ROOT/indexer/target" && ! -w "$REPO_ROOT/indexer/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-615-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi

run_docs() {
  set -euo pipefail
  rg -q "R615-1" skills/AGENTS_INDEXER_TAX_AWARE_ROUTING.md || return 1
  rg -q "COMMUNITY_TAX_OPTION2_CODE_IDS" skills/AGENTS_COMMUNITY_TAX_ROUTER.md || return 1
  rg -q "estimated_amount_out_net" docs/route-solver.md || return 1
  rg -q "#615" docs/adr/0002-global-best-execution-route-solver.md || return 1
  rg -q "tax_identity" docs/indexer-invariants.md || return 1
  rg -q "verify-issue-615" docs/testing.md || return 1
  rg -q "AGENTS_INDEXER_TAX_AWARE_ROUTING" AGENTS.md || return 1
  rg -q "AGENTS_INDEXER_TAX_AWARE_ROUTING" skills/AGENTS_HYBRID_QUOTING.md || return 1
  rg -q "AGENTS_INDEXER_TAX_AWARE_ROUTING" skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md || return 1
  rg -q "11611" indexer/src/api/community_tax_rank.rs || return 1
  rg -q "estimated_amount_out_net" indexer/src/api/route_solver.rs || return 1
  rg -q "executeAmountOut" frontend-dapp/src/utils/cw20RouteSolveQuote.ts || return 1
  # Legacy 11611 is not automatically upgraded to route-local funding.
  rg -q "COLUMBUS5_COMMUNITY_TAX_CODE_ID" indexer/src/api/community_tax_rank.rs || return 1
  rg -q "route_local_v1" indexer/src/api/community_tax_rank.rs || return 1
  rg -q "TRADING_ROUTE_TAX_CODE_IDS" indexer/src/config.rs || return 1
  rg -q "route_tax_capability" indexer/src/api/trading_tax_discovery.rs || return 1
  rg -q "1381" skills/AGENTS_INDEXER_TAX_AWARE_ROUTING.md || return 1
  rg -q "1381" docs/route-solver.md || return 1
}

run_indexer_lib() {
  (cd indexer && cargo test --lib community_tax_rank -- --test-threads=1 &&
    cargo test --lib trading_tax_discovery -- --test-threads=1 &&
    cargo test --lib funded_policy_a -- --test-threads=1 &&
    cargo test --lib route_exact_output_funded -- --test-threads=1 &&
    cargo test --lib hybrid_cache_key -- --test-threads=1 &&
    cargo test --lib merge_picks_max_net -- --test-threads=1)
}

run_frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/communityTaxNetOut.test.ts \
    src/utils/cw20RouteSolveQuote.test.ts \
    src/utils/taxPreviewMaxSpend.test.ts
}

run_607() {
  ./scripts/qa/verify-issue-607.sh
}

echo ""
echo "── first pass ──"
run_step "docs: legacy R615 + route-local v1 + ADR + skills" run_docs
run_step "indexer: discovery + funded ranking/boundaries + cache identity lib" run_indexer_lib
run_step "frontend: You Receive net + no Max double-count" run_frontend
run_step "no-regress: verify-issue-607" run_607

echo ""
echo "── retest ──"
run_step "retest docs: legacy R615 + route-local v1 + ADR + skills" run_docs
run_step "retest indexer: discovery + funded ranking/boundaries + cache identity lib" run_indexer_lib
run_step "retest frontend: You Receive net + no Max double-count" run_frontend

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #615 verification passed"
