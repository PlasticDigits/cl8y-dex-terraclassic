#!/usr/bin/env bash
# Automated verification for GitLab #662 — /pool Created relative age from indexer created_at.
#
# Proves:
#   1. PairResponse includes RFC3339 created_at on list / detail / token-pairs.
#   2. sort=created orders by pairs.created_at (default desc).
#   3. formatRelativeAge buckets + abuse cases.
#   4. /pool cell + sort click; catalog caret none; search relevance.
#   5. Docs/skills P662 + P547-4 rewrite.
#
# Refs: skills/AGENTS_FRONTEND_POOL_CREATED.md,
#       indexer/src/api/pairs.rs, frontend-dapp/src/utils/formatDate.ts
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
echo "  GitLab #662 — /pool Created relative age (indexer first-seen)"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "frontend: formatter + pool Created cell + sort" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/formatDate.test.ts \
    src/utils/__tests__/poolListQuery.test.ts \
    src/pages/PoolPage.test.tsx'

run_step "code: pair_to_response + ORDER BY created_at + relative formatter" \
  bash -c 'set -euo pipefail
    grep -qE "fn pair_to_response" indexer/src/api/pairs.rs
    grep -qE "created_at: DateTime" indexer/src/api/pairs.rs
    grep -qE "qb.push\\(\"p.created_at\"\\)" indexer/src/db/queries/pairs.rs
    grep -qE "formatRelativeAge" frontend-dapp/src/utils/formatDate.ts
    grep -qE "formatRelativeAge" frontend-dapp/src/components/pool/PoolPairsTable.tsx
    grep -qE "pool-row-created" frontend-dapp/src/components/pool/PoolPairsTable.tsx
    ! grep -qE "setInterval" frontend-dapp/src/components/pool/PoolPairsTable.tsx
    ! grep -qE "push\\(\"p.created_at_block\"\\)" indexer/src/db/queries/pairs.rs
  '

run_step "code: no per-row LCD/getPair for age (P547-9 / P662-4)" \
  bash -c 'set -euo pipefail
    ! grep -qE "getPool|getPairFeeConfig" frontend-dapp/src/components/pool/PoolPairsTable.tsx
    ! grep -qE "/gt/" frontend-dapp/src/components/pool/PoolPairsTable.tsx
    ! grep -qE "dangerouslySetInnerHTML" frontend-dapp/src/components/pool/PoolPairsTable.tsx
  '

run_step "indexer: api_pairs created_at + sort=created (Postgres)" \
  bash -c 'cd indexer && cargo test --test api_pairs -- --test-threads=1 list_pairs_returns_200 list_pairs_sort_created_orders_by_created_at get_pair_returns_pair'

run_step "indexer: token-pairs created_at (Postgres)" \
  bash -c 'cd indexer && cargo test --test api_tokens -- --test-threads=1 get_token_pairs'

run_step "docs: P662 + P547-4 rewrite + indexer first-seen" \
  bash -c 'set -euo pipefail
    grep -qE "P662-1" docs/frontend.md
    grep -qE "P662-8" docs/frontend.md
    grep -qE "P547-4" docs/frontend.md
    grep -qE "formatRelativeAge" docs/frontend.md
    grep -qE "Pair list .created_at. \\(GitLab #662\\)" docs/indexer-invariants.md
    grep -qE "verify-issue-662" docs/testing.md
    grep -qE "created_at" docs/qa-onboarding.md
  '

run_step "skill: AGENTS_FRONTEND_POOL_CREATED + crosslinks" \
  bash -c 'set -euo pipefail
    grep -qE "\\*\\*P662-1" skills/AGENTS_FRONTEND_POOL_CREATED.md
    grep -qE "\\*\\*P662-8" skills/AGENTS_FRONTEND_POOL_CREATED.md
    grep -qE "make verify-issue-662" skills/AGENTS_FRONTEND_POOL_CREATED.md
    grep -qE "AGENTS_FRONTEND_POOL_CREATED|#662" skills/AGENTS_FRONTEND_POOL_TABLE.md
    grep -qE "AGENTS_FRONTEND_POOL_CREATED|#662" skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md
    grep -qE "AGENTS_FRONTEND_POOL_CREATED|#662" skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md
    grep -qE "AGENTS_FRONTEND_POOL_CREATED|#662" AGENTS.md
  '

if [[ "${VERIFY_ISSUE_662_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright P7] skipped (VERIFY_ISSUE_662_SKIP_E2E=1)"
else
  run_step "playwright: P7 Created relative age (5 workers)" \
    bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test e2e/pool-table-547.spec.ts --project=e2e-smoke --workers=5 -g "P7 Created"'
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
