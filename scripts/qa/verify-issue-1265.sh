#!/usr/bin/env bash
# Verification for Forgejo #1265 — remaining GET /route/solve failure-mode census.
#
# Docs / process only (no LocalTerra, no solver/quote-path patch):
#   R-CENSUS-1  one ADR covering quote-path inventory + F0–F10
#   R-CENSUS-2  dated probes or explicit unmeasured rows
#   R-CENSUS-3  F0–F10 scored; Stay vs spawn
#   R-CENSUS-4  #1203 / #690 / #1222 / #1264 out of census
#   R-CENSUS-5  no production solver / quote-path diff
#   R-CENSUS-6  no impl spawn unless decision is not Stay
#   R-CENSUS-7  H596-7 pool-only native; OPTIMALITY_SCOPE top-5
#   R-CENSUS-8  contract identity; no theater paint; gems hidden
#
# Refs: docs/adr/0007-route-solve-remaining-failures.md
#       skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md
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

ADR="docs/adr/0007-route-solve-remaining-failures.md"

PROD_SOLVE_FILES=(
  indexer/src/api/best_execution.rs
  indexer/src/api/route_paths.rs
  indexer/src/api/route_solver.rs
  indexer/src/api/hybrid_route_opt.rs
  frontend-dapp/src/utils/cw20RouteSolveQuote.ts
  frontend-dapp/src/pages/SwapPage.tsx
  frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx
  frontend-dapp/src/services/indexer/client.ts
)

echo "════════════════════════════════════════════════════════════════"
echo "  Forgejo #1265 — remaining GET /route/solve failures census"
echo "════════════════════════════════════════════════════════════════"

run_adr_headings() {
  test -f "$ADR"
  grep -qE '^## Status' "$ADR"
  grep -qE '^## Context' "$ADR"
  grep -qE '^## Decision' "$ADR"
  grep -qE '^## Invariants \(R-CENSUS\)' "$ADR"
  grep -qE '^## How retail quoting works' "$ADR"
  grep -qE '^## Retail quote inventory' "$ADR"
  grep -qE '^## Dated measurements' "$ADR"
  grep -qE '^## Attack / abuse evaluation' "$ADR"
  grep -qE '^## Test plan \(research paths\)' "$ADR"
  grep -qE '\*\*Stay\*\*' "$ADR"
  grep -qE '\*\*No impl spawn\*\*' "$ADR"
  grep -qE '\*\*F0\*\*' "$ADR"
  grep -qE '\*\*F10\*\*' "$ADR"
  grep -qE '\*\*R-CENSUS-1\*\*' "$ADR"
  grep -qE '\*\*R-CENSUS-8\*\*' "$ADR"
  grep -qE '\*\*A1\*\*' "$ADR"
  grep -qE '\*\*A6\*\*' "$ADR"
  grep -qE 'issues/1203' "$ADR"
  grep -qE 'issues/690' "$ADR"
  grep -qE 'issues/1222' "$ADR"
  grep -qE 'issues/1264' "$ADR"
  grep -qE 'issues/1218' "$ADR"
  grep -qE 'issues/1257' "$ADR"
  grep -qE 'token_in not found in indexer assets' "$ADR"
  grep -qE '2026-09-21' "$ADR"
  grep -qE 'unmeasured' "$ADR"
  grep -qE 'indexer_hybrid_db' "$ADR"
  grep -qE 'global_v4' "$ADR"
  grep -qE '169635290' "$ADR"
  grep -qE '6733825127434703' "$ADR"
  grep -qE 'H596-7' "$ADR"
  grep -qE 'OPTIMALITY_SCOPE' "$ADR"
  grep -qE 'quoteCw20ViaRouteSolve' "$ADR"
  grep -qE 'make verify-issue-1265' "$ADR"
}

run_no_hybrid_on_wrap_or_split() {
  grep -qE 'Recommending hybrid native execute to “fix ranking”' "$ADR" \
    || grep -qE 'Recommending hybrid native execute to "fix ranking"' "$ADR"
  grep -qE 'other-DEX hops' "$ADR"
  grep -qE 'path-split' "$ADR"
  grep -qE '\*\*Stay\*\*' "$ADR"
  grep -qE '\*\*No impl spawn\*\*' "$ADR"
  grep -qE 'H596-7' "$ADR"
}

run_docs_crosslinks() {
  grep -qE '0007-route-solve-remaining-failures' docs/README.md
  grep -qE 'verify-issue-1265' docs/testing.md
  grep -qE 'R-CENSUS-1' docs/testing.md
  grep -qE '0007-route-solve-remaining-failures' docs/route-solver.md
  grep -qE 'Remaining failures census' docs/route-solver.md
  grep -qE '0007-route-solve-remaining-failures' docs/indexer-invariants.md
  grep -qE '0007-route-solve-remaining-failures' docs/adr/0002-global-best-execution-route-solver.md
  grep -qE 'R-CENSUS-1' skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md
  grep -qE 'R-CENSUS-8' skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md
  grep -qE 'make verify-issue-1265' skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md
  grep -qE '0007-route-solve-remaining-failures' skills/AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md
  grep -qE '0007-route-solve-remaining-failures|#1265' skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md
  grep -qE '0007-route-solve-remaining-failures|#1265' skills/AGENTS_HYBRID_QUOTING.md
  grep -qE '0007-route-solve-remaining-failures|#1265' skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md
  grep -qE '0007-route-solve-remaining-failures|#1265' skills/AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md
  grep -qE '0007-route-solve-remaining-failures|#1265' skills/AGENTS_FRONTEND_SWAP_USTR_USDT_SCALE.md
  grep -qE 'verify-issue-1265' AGENTS.md
  grep -qE 'R-CENSUS-1' AGENTS.md
  grep -qE 'R-CENSUS-1' docs/qa/issue-1265/README.md
  grep -qE 'does \*\*not\*\* change|docs-only' docs/qa/issue-1265/README.md
}

run_constants_lockstep() {
  grep -qE 'pub const MAX_PATH_CANDIDATES: usize = 5' indexer/src/api/best_execution.rs
  grep -qE 'pub\(crate\) const GET_DEFAULT_MAX_HOPS: usize = 4' indexer/src/api/route_solver.rs
  grep -qE 'const AMOUNT_CACHE_BUCKET: u128 = 1_000_000' indexer/src/api/route_solver.rs
  grep -qE 'optimal within top-5 simple paths by hop count' indexer/src/api/best_execution.rs
  grep -qE 'const ROUTE_SOLVE_TIMEOUT_MS = import.meta.env.VITE_E2E_INDEXER_OUTAGE === .1. \? 4_000 : 45_000' \
    frontend-dapp/src/services/indexer/client.ts
  grep -qE 'export async function quoteCw20ViaRouteSolve' frontend-dapp/src/utils/cw20RouteSolveQuote.ts
  grep -qE 'quoteCw20ViaRouteSolve' frontend-dapp/src/pages/SwapPage.tsx
  grep -qE 'quoteCw20ViaRouteSolve' frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx
  grep -qE 'H596-7' frontend-dapp/src/services/terraclassic/router.ts
  python3 scripts/check_route_solver_docs.py
}

run_no_prod_solve_diff() {
  local base=""
  if git rev-parse --verify origin/main >/dev/null 2>&1; then
    base="$(git merge-base HEAD origin/main)"
  elif git rev-parse --verify main >/dev/null 2>&1; then
    base="$(git merge-base HEAD main)"
  fi
  if [[ -z "$base" ]]; then
    echo "no origin/main merge-base — skip git-diff gate (constants lockstep still runs)"
    return 0
  fi
  local dirty
  dirty="$(git diff --name-only "$base" -- "${PROD_SOLVE_FILES[@]}" || true)"
  if [[ -n "$dirty" ]]; then
    echo "production solver/quote files changed vs merge-base:" >&2
    echo "$dirty" >&2
    return 1
  fi
}

echo ""
echo "── first pass ──"
run_step "ADR: required headings + F0–F10 + Stay + measurements" \
  run_adr_headings

run_step "ADR: attack rows do not recommend hybrid-on-wrap / split / other-DEX" \
  run_no_hybrid_on_wrap_or_split

run_step "docs: README + testing + route-solver + skills + AGENTS.md + QA" \
  run_docs_crosslinks

run_step "lockstep: current solver/quote constants still match census" \
  run_constants_lockstep

run_step "AC5: production solver/quote files untouched vs merge-base" \
  run_no_prod_solve_diff

echo ""
echo "── retest ──"
run_step "retest ADR headings" \
  run_adr_headings

run_step "retest AC5 production files untouched" \
  run_no_prod_solve_diff

run_step "retest constants + check_route_solver_docs.py" \
  run_constants_lockstep

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
