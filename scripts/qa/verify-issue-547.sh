#!/usr/bin/env bash
# Automated verification for GitLab #547 — /pool sortable table, catalog default, Charts deep link.
#
# Proves (unit + docs; Playwright P1–P6 when the frontend can boot):
#   1. Table not PoolCard list; no header lectures / Router-known.
#   2. Catalog default UST1-first; column sort uses indexer keys.
#   3. How-to section dismiss + #lp-howto restore.
#   4. Charts /charts/:pairAddr validation; Pool row Link href.
#   5. Skills/docs invariants P547-1–P547-10 crosslinked.
#
# Refs: skills/AGENTS_FRONTEND_POOL_TABLE.md,
#       frontend-dapp/src/components/pool/PoolPairsTable.tsx,
#       docs/frontend.md § Liquidity pools list
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
echo "  GitLab #547 — /pool sortable table + catalog default + Charts"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: pool table + howto + charts deep link + catalog" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/PoolPage.test.tsx \
    src/pages/PoolPage.feeDiscountRegistryBanner.test.tsx \
    src/pages/ChartsPage.test.tsx \
    src/components/pool/__tests__/PoolLpHowto.test.tsx \
    src/utils/__tests__/poolLpHowto.test.ts \
    src/utils/__tests__/chartsPairRoute.test.ts \
    src/utils/__tests__/poolListQuery.test.ts \
    src/utils/__tests__/pairCatalogRank.test.ts'

run_step "code: no header lectures / Router-known / PoolCard list" \
  bash -c '! grep -qE "Liquidity Pools" frontend-dapp/src/pages/PoolPage.tsx' && \
  bash -c '! grep -qE "List source" frontend-dapp/src/pages/PoolPage.tsx' && \
  bash -c '! grep -qE "Router-known|pool-filter-router|routerKnownOnly" frontend-dapp/src/pages/PoolPage.tsx' && \
  bash -c '! grep -qE "pool-fee-discount-eligibility-note" frontend-dapp/src/pages/PoolPage.tsx' && \
  bash -c '! grep -qE "const PoolCard" frontend-dapp/src/pages/PoolPage.tsx' && \
  grep -qE 'PoolPairsTable' frontend-dapp/src/pages/PoolPage.tsx && \
  grep -qE 'pool-pairs-table' frontend-dapp/src/components/pool/PoolPairsTable.tsx

run_step "code: catalog default + column sort helpers" \
  grep -qE 'catalogRankAndPaginate' frontend-dapp/src/pages/PoolPage.tsx && \
  grep -qE 'POOL_CATALOG_FETCH_LIMIT' frontend-dapp/src/utils/poolListQuery.ts && \
  grep -qE 'sortIndexerPairsByCatalog' frontend-dapp/src/utils/poolListQuery.ts && \
  grep -qE 'formatQuoteVolume24h' frontend-dapp/src/components/pool/PoolPairsTable.tsx

run_step "code: Charts deep link + Pool row Link" \
  grep -qE 'path="/charts/:pairAddr"' frontend-dapp/src/App.tsx && \
  grep -qE 'chartsPairHref' frontend-dapp/src/components/pool/PoolPairsTable.tsx && \
  grep -qE 'isChartsPairRouteParam' frontend-dapp/src/pages/ChartsPage.tsx && \
  grep -qE 'pool-row-charts' frontend-dapp/src/components/pool/PoolPairsTable.tsx

run_step "code: how-to section dismiss + hash restore" \
  grep -qE 'POOL_LP_HOWTO_SECTION_DISMISSED_KEY' frontend-dapp/src/utils/poolLpHowto.ts && \
  grep -qE 'writePoolLpHowtoSectionDismissed' frontend-dapp/src/components/pool/PoolLpHowto.tsx && \
  bash -c '! grep -qE "dangerouslySetInnerHTML|innerHTML" frontend-dapp/src/components/pool/PoolLpHowto.tsx'

run_step "code: LCD only on Manage expand (A8)" \
  grep -qE 'getPool' frontend-dapp/src/components/pool/PoolAdvancedManage.tsx && \
  bash -c '! grep -qE "getPool|getPairFeeConfig" frontend-dapp/src/pages/PoolPage.tsx' && \
  bash -c '! grep -qE "getPool|getPairFeeConfig" frontend-dapp/src/components/pool/PoolPairsTable.tsx'

run_step "docs: frontend.md P547-1–P547-10 + H531-7 section dismiss" \
  grep -qE 'P547-1' docs/frontend.md && \
  grep -qE 'P547-10' docs/frontend.md && \
  grep -qE 'pool-sortable-table|#547' docs/frontend.md && \
  grep -qE 'H531-7' docs/frontend.md && \
  grep -qE 'section dismiss|#547' docs/frontend.md

run_step "skill: AGENTS_FRONTEND_POOL_TABLE + crosslinks" \
  grep -qE '\*\*P547-1' skills/AGENTS_FRONTEND_POOL_TABLE.md && \
  grep -qE '\*\*P547-10' skills/AGENTS_FRONTEND_POOL_TABLE.md && \
  grep -qE 'make verify-issue-547' skills/AGENTS_FRONTEND_POOL_TABLE.md && \
  grep -qE 'AGENTS_FRONTEND_POOL_TABLE|#547' skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md && \
  grep -qE 'AGENTS_FRONTEND_POOL_TABLE|#547' skills/AGENTS_FRONTEND_POOL_LP_HOWTO.md && \
  grep -qE 'AGENTS_FRONTEND_POOL_TABLE|#547' skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md && \
  grep -qE 'AGENTS_FRONTEND_POOL_TABLE|#547' AGENTS.md

if [[ "${VERIFY_ISSUE_547_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright P1–P6] skipped (VERIFY_ISSUE_547_SKIP_E2E=1)"
else
  run_step "playwright: P1–P6 pool table smoke (5 workers)" \
    bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test e2e/pool-table-547.spec.ts --project=e2e-smoke --workers=5'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  Results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for line in "${RESULTS[@]}"; do
  echo "  $line"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
