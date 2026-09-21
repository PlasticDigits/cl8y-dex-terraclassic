#!/usr/bin/env bash
# Verification for Forgejo #1218 — wrap-enter native routes GET /route/solve.
#
# Layers (no LocalTerra required):
#   1. Indexer lib: skip-unusable drain helpers
#   2. Indexer integration: native uluna 400 + honest 3-hop after thin 2-hops
#   3. Frontend Vitest: wrap map, pool-only execute, Route display, Swap submit
#   4. Docs/skills H1218 + OPTIMALITY_SCOPE drift
#   5. Retest unit + docs
#
# Optional chain: wrap-swap.spec.ts E7/E8 (VERIFY_ISSUE_1218_CHAIN=1).
#
# Refs: skills/AGENTS_FRONTEND_WRAP_ENTER_ROUTE_SOLVE.md, docs/route-solver.md
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
echo "  Forgejo #1218 — wrap-enter GET /route/solve + skip-unusable K"
echo "════════════════════════════════════════════════════════════════"

run_indexer_lib() {
  (cd indexer && cargo test --lib hop_near_full_ask_drain -- --test-threads=1)
  (cd indexer && cargo test --lib path_unusable_for_top_k -- --test-threads=1)
}

run_indexer_native_400() {
  (cd indexer && cargo test --test api_route_solve \
    route_solve_native_uluna_returns_400 -- --test-threads=1)
}

run_indexer_db_hybrid() {
  (cd indexer && cargo test --test api_route_solve_db_hybrid \
    route_solve_db_hybrid_skip_unusable_admits_honest_3hop -- --test-threads=1)
}

run_frontend_unit() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/utils/nativeWrapRouteSolve.test.ts \
    src/utils/hybridHopOfferPartition.test.ts \
    src/utils/swapRouteDisplay.test.ts \
    src/services/terraclassic/router.test.ts \
    src/pages/SwapPage.test.tsx
}

run_docs() {
  set -euo pipefail
  python3 scripts/check_route_solver_docs.py
  rg -q 'PATH_ENUM_POOL' indexer/src/api/best_execution.rs
  rg -q 'select_usable_top_k' indexer/src/api/best_execution.rs
  rg -q 'path_unusable_for_top_k' indexer/src/api/db_orderbook_sim.rs
  rg -q 'hop_near_full_ask_drain' indexer/src/api/db_orderbook_sim.rs
  rg -q 'route_solve_native_uluna_returns_400' indexer/tests/api_route_solve.rs
  rg -q 'route_solve_db_hybrid_skip_unusable_admits_honest_3hop' indexer/tests/api_route_solve_db_hybrid.rs
  rg -q 'wrapMappedSolvePair' frontend-dapp/src/utils/nativeWrapRouteSolve.ts
  rg -q 'omitAllHybrid' frontend-dapp/src/utils/cw20RouteSolveQuote.ts
  rg -q 'poolOnlyNativeExecuteOps' frontend-dapp/src/services/terraclassic/router.ts
  rg -q 'stripAllDeclaredHybrid' frontend-dapp/src/utils/hybridHopOfferPartition.ts
  rg -q 'H1218-1' skills/AGENTS_FRONTEND_WRAP_ENTER_ROUTE_SOLVE.md
  rg -q '#1218' skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md
  rg -q 'H1280-6' skills/AGENTS_HYBRID_HOP_OFFER.md
  rg -q '#1218' skills/AGENTS_HYBRID_QUOTING.md
  rg -q '#1218' skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md
  rg -q '#1218' skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md
  rg -q 'skip ~100% hop-spread' docs/adr/0002-global-best-execution-route-solver.md
  rg -q 'PATH_ENUM_POOL' docs/route-solver.md
  rg -q 'verify-issue-1218' AGENTS.md
  rg -q 'AGENTS_FRONTEND_WRAP_ENTER_ROUTE_SOLVE' AGENTS.md
  rg -q 'verify-issue-1218' docs/testing.md
  rg -q '#1218' docs/indexer-invariants.md
  rg -q '#1218' docs/frontend.md
  rg -q '#1218' docs/integrators.md
}

echo ""
echo "── first pass ──"
run_step "indexer lib: skip-unusable hop helpers" run_indexer_lib
run_step "indexer integration: native uluna 400 (AC8)" run_indexer_native_400
run_step "indexer integration: skip-unusable admits 3-hop (AC7)" run_indexer_db_hybrid
run_step "frontend unit: wrap map + execute + Swap" run_frontend_unit
run_step "docs: H1218 + OPTIMALITY_SCOPE + skills" run_docs

echo ""
echo "── retest ──"
run_step "retest indexer lib" run_indexer_lib
run_step "retest indexer native uluna 400" run_indexer_native_400
run_step "retest indexer skip-unusable 3-hop" run_indexer_db_hybrid
run_step "retest frontend unit" run_frontend_unit
run_step "retest docs" run_docs

if [ "${VERIFY_ISSUE_1218_CHAIN:-0}" = "1" ]; then
  if make has-localterra >/dev/null 2>&1 && [ -f frontend-dapp/.env.local ]; then
    run_step "chain: Playwright wrap-swap E7/E8" \
      bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- \
        ./node_modules/.bin/playwright test e2e/wrap-swap.spec.ts --project=e2e-tx'
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
echo "==> Forgejo #1218 verification passed"
