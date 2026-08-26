#!/usr/bin/env bash
# Automated verification for GitLab #655 — /pool v2 LP USD + pair_liquidity_usd rollup.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env) for indexer tests.
# Playwright e2e-smoke is optional when frontend-dapp/node_modules exists.
#
# Refs: skills/AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md,
#       skills/AGENTS_FRONTEND_POOL_TABLE.md,
#       docs/frontend.md Liquidity pools list (P655),
#       docs/indexer-invariants.md pair-list liquidity USD
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
  grep -qE "P655-1" docs/frontend.md
  grep -qE "P655-8" docs/frontend.md
  grep -qE "Pair list v2 LP USD" docs/indexer-invariants.md
  grep -qE "pair_liquidity_usd" docs/indexer-invariants.md
  grep -qE "make verify-issue-655" skills/AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md
  grep -qE "make verify-issue-655" skills/AGENTS_FRONTEND_POOL_TABLE.md
  grep -qE "P655-1" skills/AGENTS_FRONTEND_POOL_TABLE.md
  grep -qE "AGENTS_INDEXER_PAIR_LIQUIDITY_USD" AGENTS.md
  grep -qE "verify-issue-655" AGENTS.md
  grep -qE "verify-issue-655" docs/testing.md
  grep -qE "pair_liquidity_usd" docs/runbooks/overview-global-stats-brin.md
  test -f indexer/migrations/20260826150000_pair_liquidity_usd.sql
}

source_guards() {
  grep -qE "liquidity_usd" indexer/src/api/pairs.rs
  grep -qE "PairListSort::LiquidityUsd" indexer/src/db/queries/pairs.rs
  grep -qE "LEFT JOIN pair_liquidity_usd" indexer/src/db/queries/pairs.rs
  grep -qE "pair_liquidity_usd::replace_pair_liquidity_usd" indexer/src/indexer/protocol_tvl.rs
  grep -qE "pool-sort-lp-usd" frontend-dapp/src/components/pool/PoolPairsTable.tsx
  grep -qE "formatProtocolUsd" frontend-dapp/src/components/pool/PoolPairsTable.tsx
  grep -qE "colSpan=\{7\}" frontend-dapp/src/components/pool/PoolPairsTable.tsx
  grep -qE "liquidity_usd" frontend-dapp/src/utils/poolListQuery.ts
  if grep -nE "getPool|getPairFeeConfig" frontend-dapp/src/components/pool/PoolPairsTable.tsx \
       frontend-dapp/src/pages/PoolPage.tsx 2>/dev/null; then
    echo "Pool table/page must not call getPool / getPairFeeConfig" >&2
    return 1
  fi
  if grep -nF "liquidity_in_usd" frontend-dapp/src/components/pool/PoolPairsTable.tsx \
       frontend-dapp/src/pages/PoolPage.tsx 2>/dev/null; then
    echo "Pool UI must not read CG liquidity_in_usd" >&2
    return 1
  fi
}

indexer_lib() {
  (cd indexer && cargo test --lib protocol_tvl -- --quiet)
}

indexer_integration() {
  (cd indexer && cargo test --test indexer_pair_liquidity_usd --test api_pairs --test indexer_protocol_liquidity -- --test-threads=1 --quiet)
}

frontend_vitest() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/PoolPage.test.tsx \
    src/utils/__tests__/poolListQuery.test.ts \
    src/utils/__tests__/formatProtocolStats.test.ts
}

playwright_pool_table() {
  # Dedicated :3173 is on LocalTerra indexer CORS (#625 / M673-7). Do not reuse :3000
  # from another worktree (CORS miss → no pool-pairs-table).
  PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-3173}" \
    PLAYWRIGHT_BASE_URL="http://127.0.0.1:${PLAYWRIGHT_WEB_PORT:-3173}" \
    bash scripts/with-node.sh --cwd frontend-dapp -- \
    ./node_modules/.bin/playwright test e2e/pool-table-547.spec.ts --project=e2e-smoke --workers=5
}

echo "================================================================"
echo "  GitLab #655 — /pool v2 LP USD + pair-list liquidity rollup"
echo "================================================================"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs: invariants + skills + AGENTS crosslinks" docs_crosslinks
run_step "source: rollup JOIN; no GET-path reserves; no LCD on table" source_guards
run_step "indexer lib: protocol_tvl math" indexer_lib
run_step "indexer integration: pair liquidity stamp + list + sort + 400" indexer_integration
run_step "frontend: PoolPage + poolListQuery + formatProtocolUsd" frontend_vitest

if [ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]; then
  COMMON="$(git rev-parse --git-common-dir)"
  MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
  if [ -f "$MAIN_ROOT/frontend-dapp/.env.local" ] && [ "$MAIN_ROOT" != "$REPO_ROOT" ]; then
    cp "$MAIN_ROOT/frontend-dapp/.env.local" "$REPO_ROOT/frontend-dapp/.env.local"
    echo ""
    echo "[bootstrap] copied frontend-dapp/.env.local from primary checkout (gitignored)"
  fi
fi

if [[ "${VERIFY_ISSUE_655_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright P1-P7] skipped (VERIFY_ISSUE_655_SKIP_E2E=1)"
  ok "playwright pool-table (skipped)"
elif [ -d "$REPO_ROOT/frontend-dapp/node_modules/@playwright/test" ] || [ -d "$REPO_ROOT/frontend-dapp/node_modules/playwright" ]; then
  run_step "playwright: pool table P1-P7 (5 workers)" playwright_pool_table
else
  echo ""
  echo "[playwright pool-table] SKIP (no Playwright install)"
  ok "playwright pool-table (skipped — no node_modules playwright)"
fi

if [[ "${VERIFY_ISSUE_655_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related 547/569] skipped (VERIFY_ISSUE_655_SKIP_RELATED=1)"
  ok "related verifies (skipped)"
else
  run_step "related: verify-issue-547 (no nested e2e)" \
    env VERIFY_ISSUE_547_SKIP_E2E=1 make verify-issue-547
  run_step "related: verify-issue-569 (no nested e2e/related)" \
    env VERIFY_ISSUE_569_SKIP_E2E=1 VERIFY_ISSUE_569_SKIP_RELATED=1 make verify-issue-569
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
