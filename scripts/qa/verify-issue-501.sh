#!/usr/bin/env bash
# Automated verification for GitLab #501 — Trade market defaults to GET /route/solve.
#
# Proves (unit + docs; optional chain steps when LocalTerra + indexer are up):
#   1. Shared quoteCw20ViaRouteSolve uses GET /route/solve + wallet sim.
#   2. TradeMarketOrderPanel: default GET (no POST); Advanced typed book → POST;
#      submit uses solver hybrid + submit cap. No hybrid opt-out (#596).
#   3. Empty manual book is not 100% book (getDirectHybridBookSplit).
#   4. Docs/skills/ADR crosslinks for #501 remain present.
#
# Optional (VERIFY_ISSUE_501_CHAIN=1): Network/UI smoke via Playwright when
#   make has-localterra succeeds and frontend-dapp/.env.local exists.
#
# Refs: frontend-dapp/src/utils/cw20RouteSolveQuote.ts,
#       frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx,
#       docs/limit-orders.md, skills/AGENTS_HYBRID_QUOTING.md,
#       skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md,
#       docs/adr/0002-global-best-execution-route-solver.md
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
echo "  GitLab #501 — Trade market GET /route/solve default"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: cw20RouteSolveQuote + TradeMarketOrderPanel + swapDisclosure" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/cw20RouteSolveQuote.test.ts \
    src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx \
    src/utils/swapDisclosure.test.ts \
    src/utils/directHybridQuote.test.ts'

run_step "docs: limit-orders empty manual book + GET default (#501)" \
  grep -qE 'Empty manual book \(Swap \+ Trade, \[#501\]' docs/limit-orders.md

run_step "docs: ADR 0002 #501 amendment" \
  grep -qE '\*\*#501\*\*' docs/adr/0002-global-best-execution-route-solver.md

run_step "docs: indexer-invariants retail GET default (#501)" \
  grep -qE 'Retail GET default \(Swap \+ Trade market, \[#501\]' docs/indexer-invariants.md

run_step "skill: AGENTS_HYBRID_QUOTING Swap + Trade GET (#501)" \
  grep -qE 'Default \(Swap \+ Trade market\).*GET /route/solve' skills/AGENTS_HYBRID_QUOTING.md

run_step "skill: AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY Trade GET (#501)" \
  grep -qE 'quoteCw20ViaRouteSolve.*GET /route/solve' skills/AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md

run_step "skill: AGENTS.md playbook links #501" \
  grep -qE 'Trade market GET .*/route/solve.* default \(#501\)' AGENTS.md

run_step "code: Trade submit applies hybridParamsWithSubmitCap on GET hybrid" \
  grep -qE 'hybridParamsWithSubmitCap\(hopHybrid\)' \
    frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx

run_step "e2e: fee-discount-quote-245.spec.ts present" \
  test -f frontend-dapp/e2e/fee-discount-quote-245.spec.ts

run_step "e2e: trade-market-route-solve-501-tx.spec.ts present" \
  test -f frontend-dapp/e2e/trade-market-route-solve-501-tx.spec.ts

if [ "${VERIFY_ISSUE_501_CHAIN:-0}" = "1" ]; then
  if make has-localterra >/dev/null 2>&1 && [ -f frontend-dapp/.env.local ]; then
    run_step "chain: Playwright fee-discount Trade pool-only path (#245 selector)" \
      bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- \
        ./node_modules/.bin/playwright test e2e/fee-discount-quote-245.spec.ts \
        --project=e2e-tx -g "Trade market"'
  else
    echo ""
    echo "[chain] SKIP — LocalTerra or frontend-dapp/.env.local not ready"
    echo "  Provision: make setup-cloud-localterra"
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
echo "==> GitLab #501 verification passed"
