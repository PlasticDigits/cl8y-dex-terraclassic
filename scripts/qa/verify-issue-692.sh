#!/usr/bin/env bash
# Automated verification for GitLab #692 — /pool Vol USD + pair_volume_24h.volume_usd.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env) for indexer tests.
# Playwright e2e-smoke is optional when frontend-dapp/node_modules exists.
#
# Refs: skills/AGENTS_INDEXER_PAIR_VOLUME_USD.md,
#       skills/AGENTS_FRONTEND_POOL_TABLE.md,
#       skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md,
#       docs/frontend.md Liquidity pools list (PVol),
#       docs/indexer-invariants.md pair list 24h volume
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

docs_crosslinks() {
  grep -qE "PVol-1" docs/frontend.md
  grep -qE "PVol-8" docs/frontend.md
  grep -qE "volume_usd_24h" docs/indexer-invariants.md
  grep -qE "make verify-issue-692" skills/AGENTS_INDEXER_PAIR_VOLUME_USD.md
  grep -qE "make verify-issue-692" skills/AGENTS_FRONTEND_POOL_TABLE.md
  grep -qE "PVol-1" skills/AGENTS_FRONTEND_POOL_TABLE.md
  grep -qE "AGENTS_INDEXER_PAIR_VOLUME_USD" AGENTS.md
  grep -qE "verify-issue-692" AGENTS.md
  grep -qE "verify-issue-692" docs/testing.md
  test -f indexer/migrations/20260829120000_pair_volume_24h_usd.sql
}

source_guards() {
  grep -qE "volume_usd_24h" indexer/src/api/pairs.rs
  grep -qE "PairListSort::VolumeUsd24h" indexer/src/db/queries/pairs.rs
  grep -qE "pv.volume_usd AS volume_usd_24h" indexer/src/db/queries/pairs.rs
  grep -qE "volume_usd" indexer/src/db/queries/volume.rs
  grep -qE "formatPairListVolumeUsd" frontend-dapp/src/components/pool/PoolPairsTable.tsx
  grep -qE "volume_usd_24h" frontend-dapp/src/utils/poolListQuery.ts
  grep -qE "colSpan=\{7\}" frontend-dapp/src/components/pool/PoolPairsTable.tsx
  grep -qE "formatPairListVolumeUsd" frontend-dapp/src/components/trade/PairSearchSelect.tsx
  if grep -nE "formatQuoteVolume24h" frontend-dapp/src/components/pool/PoolPairsTable.tsx \
       frontend-dapp/src/components/pool/PoolAdvancedManage.tsx 2>/dev/null; then
    echo "Pool Vol/Manage must not format quote-token volume" >&2
    return 1
  fi
  if grep -nE "getPool|getPairFeeConfig" frontend-dapp/src/components/pool/PoolPairsTable.tsx \
       frontend-dapp/src/pages/PoolPage.tsx 2>/dev/null; then
    echo "Pool table/page must not call getPool / getPairFeeConfig" >&2
    return 1
  fi
}

indexer_integration() {
  (cd indexer && cargo test --test indexer_pair_volume_usd --test indexer_pair_volume_pagination --test api_pairs -- --test-threads=1 --quiet)
}

frontend_vitest() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/PoolPage.test.tsx \
    src/utils/__tests__/poolListQuery.test.ts \
    src/utils/__tests__/pairCatalogRank.test.ts \
    src/utils/__tests__/chartsOverviewStats.test.ts \
    src/utils/__tests__/trailingWindowCopy.test.ts \
    src/components/trade/__tests__/PairSearchSelect.issue534.test.tsx
}

playwright_pool_table() {
  PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-3173}" \
    PLAYWRIGHT_BASE_URL="http://127.0.0.1:${PLAYWRIGHT_WEB_PORT:-3173}" \
    bash scripts/with-node.sh --cwd frontend-dapp -- \
    ./node_modules/.bin/playwright test e2e/pool-table-547.spec.ts --project=e2e-smoke --workers=5
}

echo "================================================================"
echo "  GitLab #692 — /pool Vol USD + pair-list 24h volume stamp"
echo "================================================================"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs: invariants + skills + AGENTS crosslinks" docs_crosslinks
run_step "source: rollup JOIN; USD Vol cell; no quote fallback on /pool" source_guards
run_step "indexer integration: volume_usd stamp + list + sort + 400" indexer_integration
run_step "frontend: PoolPage + catalog + badges + format" frontend_vitest

if [ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]; then
  COMMON="$(git rev-parse --git-common-dir)"
  MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
  if [ -f "$MAIN_ROOT/frontend-dapp/.env.local" ] && [ "$MAIN_ROOT" != "$REPO_ROOT" ]; then
    cp "$MAIN_ROOT/frontend-dapp/.env.local" "$REPO_ROOT/frontend-dapp/.env.local"
    echo ""
    echo "[bootstrap] copied frontend-dapp/.env.local from primary checkout (gitignored)"
  fi
fi

if [[ "${VERIFY_ISSUE_692_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright PVol] skipped (VERIFY_ISSUE_692_SKIP_E2E=1)"
  ok "playwright pool-table (skipped)"
elif [ -d "$REPO_ROOT/frontend-dapp/node_modules/@playwright/test" ] || [ -d "$REPO_ROOT/frontend-dapp/node_modules/playwright" ]; then
  run_step "playwright: pool table Vol USD (5 workers)" playwright_pool_table
else
  echo ""
  echo "[playwright pool-table] SKIP (no Playwright install)"
  ok "playwright pool-table (skipped — no node_modules playwright)"
fi

if [[ "${VERIFY_ISSUE_692_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related 547/655/534] skipped (VERIFY_ISSUE_692_SKIP_RELATED=1)"
  ok "related verifies (skipped)"
else
  run_step "related: verify-issue-547 (no nested e2e)" \
    env VERIFY_ISSUE_547_SKIP_E2E=1 make verify-issue-547
  run_step "related: verify-issue-655 (no nested e2e/related)" \
    env VERIFY_ISSUE_655_SKIP_E2E=1 VERIFY_ISSUE_655_SKIP_RELATED=1 make verify-issue-655
  run_step "related: verify-issue-534" \
    make verify-issue-534
fi

echo ""
echo "================================================================"
echo "  Results: ${PASS} passed, ${FAIL} failed"
echo "================================================================"
for line in "${RESULTS[@]}"; do
  echo "  $line"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
