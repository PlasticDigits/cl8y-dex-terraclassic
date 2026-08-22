#!/usr/bin/env bash
# Automated verification for GitLab #596 — retail hybrid always-on (no opt-out).
#
# Proves (unit + docs; optional chain when LocalTerra + indexer are up):
#   1. Swap + Trade have no hybrid on/off checkbox.
#   2. Default CW20 quotes still use GET /route/solve (empty book).
#   3. Advanced typed book still uses POST override.
#   4. Docs/skills/ADR crosslinks for #596 remain present.
#
# Optional (VERIFY_ISSUE_596_CHAIN=1): Playwright when make has-localterra
# succeeds and frontend-dapp/.env.local exists.
#
# Refs: skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md,
#       frontend-dapp/src/components/swap/SwapAdvancedSettings.tsx,
#       frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx,
#       docs/limit-orders.md, skills/AGENTS_HYBRID_QUOTING.md
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
echo "  GitLab #596 — retail hybrid always-on (no opt-out)"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: TradeMarketOrderPanel + swapDisclosure + SwapPage" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx \
    src/utils/swapDisclosure.test.ts \
    src/pages/SwapPage.test.tsx'

run_step "code: Swap Advanced has no hybrid checkbox" \
  bash -c '! grep -qE "Route part of input through the limit book|onUseHybridBookChange" \
    frontend-dapp/src/components/swap/SwapAdvancedSettings.tsx'

run_step "code: Trade market has no hybrid toggle" \
  bash -c '! grep -qE "trade-market-hybrid-toggle|useHybridBook" \
    frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx'

run_step "code: SwapPage has no useHybridBook state" \
  bash -c '! grep -qE "useHybridBook" frontend-dapp/src/pages/SwapPage.tsx'

run_step "docs: limit-orders always-on hybrid (#596)" \
  grep -qE '\[#596\]' docs/limit-orders.md

run_step "docs: ADR 0002 #596 amendment" \
  grep -qE '\*\*#596\*\*' docs/adr/0002-global-best-execution-route-solver.md

run_step "docs: indexer-invariants always-on (#596)" \
  grep -qE 'always-on \[#596\]' docs/indexer-invariants.md

run_step "skill: AGENTS_FRONTEND_HYBRID_ALWAYS_ON H596 invariants" \
  grep -qE 'H596-1' skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md

run_step "skill: AGENTS_HYBRID_QUOTING always on (#596)" \
  grep -qE 'always on.*#596|#596.*always on' skills/AGENTS_HYBRID_QUOTING.md

run_step "skill: AGENTS.md playbook links #596" \
  grep -qE 'AGENTS_FRONTEND_HYBRID_ALWAYS_ON' AGENTS.md

run_step "skill: AGENTS.md verify-issue-596" \
  grep -qE 'verify-issue-596' AGENTS.md

if [ "${VERIFY_ISSUE_596_CHAIN:-0}" = "1" ]; then
  if make has-localterra >/dev/null 2>&1 && [ -f frontend-dapp/.env.local ]; then
    run_step "chain: Playwright hybrid-swap UI (book override, no checkbox)" \
      bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- \
        ./node_modules/.bin/playwright test e2e/hybrid-swap.spec.ts \
        --project=e2e-tx -g "shows hybrid book disclosure"'
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
echo "==> GitLab #596 verification passed"
