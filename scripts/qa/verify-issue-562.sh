#!/usr/bin/env bash
# Automated verification for GitLab #562 — hide soft-launch gems from production retail UI.
#
# Proves (unit + docs; Playwright local path when LocalTerra is up):
#   1. retailExposeTestTokens + hardcoded columbus-5 gem addrs; UST1 is not a gem.
#   2. Swap/Trade/Pool/Create filters omit gems on mainnet; local still lists them.
#   3. findRoute / route/solve reject gem hops between economic tokens on production.
#   4. Docs/skills P562 invariants + Coolify faucet unset + F9; no ?showGems= query gate.
#
# Refs: skills/AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md,
#       frontend-dapp/src/utils/pairCatalogRank.ts,
#       docs/frontend.md § Production hide of test tokens
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
echo "  GitLab #562 — hide soft-launch gems from production retail UI"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: expose flag + filters + pickers + routes (U1–U10)" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/pairCatalogRank.test.ts \
    src/utils/__tests__/pairCatalogRank.issue562.test.ts \
    src/utils/__tests__/tokenSearchQuery.test.ts \
    src/utils/__tests__/pairSearchQuery.test.ts \
    src/utils/__tests__/poolListQuery.test.ts \
    src/utils/__tests__/createPairTokenCatalog.test.ts \
    src/components/trade/__tests__/PairSearchSelect.issue534.test.tsx \
    src/components/trade/__tests__/PairSearchSelect.issue562.test.tsx \
    src/services/terraclassic/router.test.ts \
    src/utils/cw20RouteSolveQuote.test.ts \
    src/pages/SwapPage.test.tsx \
    src/pages/PoolPage.test.tsx \
    src/pages/TradePage.test.tsx'

run_step "code: hardcoded columbus-5 gem addrs include QUARTZ/PEARL" \
  bash -c 'grep -qE "terra17dpnjlpgsnm8muu4msfjra4f2hrptnjp2jdpkka4p0e3px42ayxq0pmc2z" frontend-dapp/src/utils/pairCatalogRank.ts &&
    grep -qE "terra18fzufz8cs7ez49xjwgs248x85za5v50yug55fj7lyxp9hapxyr7qnh3czs" frontend-dapp/src/utils/pairCatalogRank.ts &&
    grep -qE "retailExposeTestTokens" frontend-dapp/src/utils/pairCatalogRank.ts &&
    grep -qE "COLUMBUS5_GEM_ADDRESSES" frontend-dapp/src/utils/pairCatalogRank.ts &&
    grep -qE "shouldRejectGemBridgeQuote" frontend-dapp/src/utils/pairCatalogRank.ts'

run_step "code: pickers + pool + create + swap defaults call the hide helpers" \
  bash -c 'grep -qE "filterRetailDiscoveryTokens" frontend-dapp/src/utils/tokenSearchQuery.ts &&
    grep -qE "filterRetailDiscoveryPairInfos" frontend-dapp/src/utils/pairSearchQuery.ts &&
    grep -qE "filterRetailDiscoveryPairInfos" frontend-dapp/src/components/trade/PairSearchSelect.tsx &&
    grep -qE "filterRetailDiscoveryIndexerPairs" frontend-dapp/src/pages/ChartsPage.tsx &&
    grep -qE "filterPoolIndexerPairs" frontend-dapp/src/pages/PoolPage.tsx &&
    grep -qE "retailExposeTestTokens" frontend-dapp/src/utils/createPairTokenCatalog.ts &&
    grep -qE "defaultRetailSwapTokenPair" frontend-dapp/src/pages/SwapPage.tsx &&
    grep -qE "shouldRejectGemBridgeQuote" frontend-dapp/src/utils/cw20RouteSolveQuote.ts &&
    grep -qE "routingGraphPairs|retailExposeTestTokens" frontend-dapp/src/services/terraclassic/router.ts'

run_step "code: VITE_SHOW_TEST_TOKENS is build-arg only (X8)" \
  bash -c 'grep -qE "VITE_SHOW_TEST_TOKENS" frontend-dapp/src/vite-env.d.ts &&
    grep -qE "VITE_SHOW_TEST_TOKENS" docker/frontend/Dockerfile &&
    ! grep -qE "showGems|searchParams.*SHOW_TEST" frontend-dapp/src/utils/pairCatalogRank.ts &&
    ! grep -qE "showGems" frontend-dapp/src/pages/SwapPage.tsx'

run_step "docs: frontend.md P562-1–P562-8" \
  bash -c 'grep -qE "production-hide-test-tokens" docs/frontend.md &&
    grep -qE "\\*\\*P562-1\\*\\*" docs/frontend.md &&
    grep -qE "\\*\\*P562-8\\*\\*" docs/frontend.md'

run_step "skill: AGENTS_FRONTEND_RETAIL_TEST_TOKENS + verify-issue-562" \
  bash -c 'grep -qE "\\*\\*P562-1" skills/AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md &&
    grep -qE "make verify-issue-562" skills/AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md &&
    grep -qE "retailExposeTestTokens" skills/AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md'

run_step "crosslinks: catalog rank + token search + create pair + pool + faucet + AGENTS.md" \
  bash -c 'grep -qE "AGENTS_FRONTEND_RETAIL_TEST_TOKENS|#562" skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md &&
    grep -qE "AGENTS_FRONTEND_RETAIL_TEST_TOKENS|#562" skills/AGENTS_FRONTEND_TOKEN_SEARCH.md &&
    grep -qE "AGENTS_FRONTEND_RETAIL_TEST_TOKENS|#562" skills/AGENTS_FRONTEND_CREATE_PAIR_PICKER.md &&
    grep -qE "AGENTS_FRONTEND_RETAIL_TEST_TOKENS|#562" skills/AGENTS_FRONTEND_POOL_TABLE.md &&
    grep -qE "AGENTS_FRONTEND_RETAIL_TEST_TOKENS|#562" skills/AGENTS_SOFT_LAUNCH_FAUCET.md &&
    grep -qE "AGENTS_FRONTEND_RETAIL_TEST_TOKENS|#562" skills/AGENTS_HYBRID_QUOTING.md &&
    grep -qE "AGENTS_FRONTEND_RETAIL_TEST_TOKENS|#562" AGENTS.md'

run_step "runbook: production Coolify unsets faucet + F9 Pause (#562 A6)" \
  bash -c 'grep -qE "VITE_FAUCET_ADDRESS" docs/runbooks/soft-launch-faucet.md &&
    grep -qE "#562|production hide|dex.cl8y.com" docs/runbooks/soft-launch-faucet.md &&
    grep -qE "Pause" docs/runbooks/soft-launch-faucet.md &&
    grep -qE "VITE_SHOW_TEST_TOKENS" frontend-dapp/.env.example &&
    grep -qE "VITE_SHOW_TEST_TOKENS" docker/frontend/Dockerfile &&
    grep -qE "Do not set \`VITE_FAUCET_ADDRESS\` on production" docs/runbooks/mainnet-soft-launch.md'

if make has-localterra >/dev/null 2>&1 && [[ -f frontend-dapp/.env.local ]]; then
  if [[ -f frontend-dapp/e2e/issue-158-swap-route.spec.ts ]]; then
    run_step "playwright: LocalTerra gems still in Swap pay list (P1 / A7)" \
      bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/issue-158-swap-route.spec.ts --grep "pay picker lists every factory token including gems"'
  else
    echo ""
    echo "[playwright: LocalTerra gems still in Swap pay list (P1 / A7)]"
    echo "  SKIP (P1 spec not in tree) — add e2e coverage that LocalTerra Swap pay still lists EMBER"
  fi
else
  echo ""
  echo "[playwright: LocalTerra gems still in Swap pay list (P1 / A7)]"
  echo "  SKIP (no LocalTerra deploy env) — P1 needs a running chain and frontend-dapp/.env.local"
fi

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
