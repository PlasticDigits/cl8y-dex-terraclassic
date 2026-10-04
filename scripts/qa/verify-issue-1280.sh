#!/usr/bin/env bash
# Verification for Forgejo #1280 — declared hybrid split equals the economic hop offer.
# Route-local v1 (#1381): gross includes sell tax; partitions use funded economic input.
# Legacy/ordinary zero-tax split tests remain separate compatibility/security regressions.
#
# Layers (no LocalTerra required):
#   1. Indexer unit: hop0-only retail plan + funded economic/net propagation
#   2. Frontend Vitest: strip interior GET hybrid, hop0 preflight, wrap hops omit hybrid
#   3. Docs/skills H1280 crosslinks
#   4. Retest unit + docs
#
# Optional chain: existing e2e/multihop-hybrid-tx.spec.ts (VERIFY_ISSUE_1280_CHAIN=1).
#
# Refs: skills/AGENTS_HYBRID_HOP_OFFER.md, docs/integrators.md, docs/route-solver.md
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
echo "  Forgejo #1280 — declared hybrid hop-offer partition (Policy A)"
echo "════════════════════════════════════════════════════════════════"

run_indexer_unit() {
  (cd indexer && cargo test --lib retail_plan_keeps_hop0 -- --test-threads=1) || return 1
  (cd indexer && cargo test --lib hop0_only_strip_recomputes_grid_out -- --test-threads=1) || return 1
  (cd indexer && cargo test --lib funded_policy_a -- --test-threads=1)
}

run_indexer_db_hybrid() {
  (cd indexer && cargo test --test api_route_solve_db_hybrid \
    route_solve_db_hybrid_2hop_live_book_fidelity -- --test-threads=1)
}

run_wasm_multitest() {
  (
    cd smartcontracts
    cargo test -p cl8y-dex-tests -- --test-threads=1 router_declared_split_mismatch_reverts_hop0 || exit 1
    cargo test -p cl8y-dex-tests -- --test-threads=1 router_two_hop_interior_hybrid_mismatch_reverts || exit 1
    cargo test -p dex-common --lib route_tax -- --test-threads=1
  )
}

run_frontend_unit() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/utils/hybridHopOfferPartition.test.ts \
    src/utils/cw20RouteSolveQuote.test.ts \
    src/services/terraclassic/router.hybrid.test.ts \
    src/utils/__tests__/humanizeTerraTxError.test.ts
}

run_docs() {
  set -euo pipefail
  rg -q 'retail_declared_hybrid_plan_hop0_only' indexer/src/api/hybrid_route_opt.rs || return 1
  rg -q 'retail_declared_hybrid_plan_hop0_only' indexer/src/api/best_execution.rs || return 1
  rg -q 'propagate_offer_through_plan' indexer/src/api/best_execution.rs || return 1
  rg -q 'route_solve_db_hybrid_2hop_live_book_fidelity' indexer/tests/api_route_solve_db_hybrid.rs || return 1
  rg -q 'optimize_funded_route' indexer/src/api/hybrid_route_opt.rs || return 1
  rg -q 'funded_policy_a' indexer/src/api/hybrid_route_opt.rs || return 1
  rg -q '1381' docs/route-solver.md || return 1
  rg -q 'stripInteriorDeclaredHybrid' frontend-dapp/src/utils/cw20RouteSolveQuote.ts || return 1
  rg -q 'assertHop0DeclaredHybridPartitionsOffer' frontend-dapp/src/services/terraclassic/router.ts || return 1
  rg -q 'never copy hybrid' frontend-dapp/src/services/terraclassic/router.ts || return 1
  rg -q '#1280' skills/AGENTS_HYBRID_HOP_OFFER.md || return 1
  rg -q 'H1280-1' skills/AGENTS_HYBRID_HOP_OFFER.md || return 1
  rg -q 'hop 0 only' docs/integrators.md || return 1
  rg -q '#1280' docs/route-solver.md || return 1
  rg -q 'H1280' docs/contracts-security-audit.md || return 1
  rg -q 'verify-issue-1280' AGENTS.md || return 1
  rg -q 'AGENTS_HYBRID_HOP_OFFER' AGENTS.md || return 1
  rg -q 'verify-issue-1280' docs/testing.md || return 1
  rg -q '#1280' skills/AGENTS_HYBRID_QUOTING.md || return 1
  rg -q '#1280' skills/AGENTS_TESTING_MULTIHOP_HYBRID.md || return 1
  rg -q '#1280' skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md || return 1
  rg -q '#1280' docs/adr/0001-hybrid-quoting-and-routing.md || return 1
}

echo ""
echo "── first pass ──"
run_step "indexer unit: hop0-only retail plan" run_indexer_unit
run_step "indexer integration: DB 2-hop stripped-plan fidelity" run_indexer_db_hybrid
run_step "wasm multitest: hop0 + interior split mismatch reverts" run_wasm_multitest
run_step "frontend unit: strip + hop0 preflight + humanize" run_frontend_unit
run_step "docs: #1280 + H1280 + skills" run_docs

echo ""
echo "── retest ──"
run_step "retest indexer unit" run_indexer_unit
run_step "retest indexer DB 2-hop fidelity" run_indexer_db_hybrid
run_step "retest wasm multitest" run_wasm_multitest
run_step "retest frontend unit" run_frontend_unit
run_step "retest docs" run_docs

if [ "${VERIFY_ISSUE_1280_CHAIN:-0}" = "1" ]; then
  if make has-localterra >/dev/null 2>&1 && [ -f frontend-dapp/.env.local ]; then
    run_step "chain: Playwright multihop-hybrid-tx" \
      bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- \
        ./node_modules/.bin/playwright test e2e/multihop-hybrid-tx.spec.ts --project=e2e-tx'
  else
    echo ""
    echo "[chain] SKIP — LocalTerra or frontend-dapp/.env.local not ready"
  fi
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
echo "==> Forgejo #1280 verification passed"
