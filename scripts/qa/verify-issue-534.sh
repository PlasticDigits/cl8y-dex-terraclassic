#!/usr/bin/env bash
# Automated verification for GitLab #534 — pair catalog rank + human quote volume.
#
# Proves (unit + docs; no chain required):
#   1. Economic pairs group ahead of gems; UST1 markets sit together.
#   2. Human quote-volume compare is mixed-decimal safe; USTR vol badge is not T.
#   3. PairSearchSelect empty browse + Test pairs divider + Trade auto-pick helper.
#   4. Docs/skills/invariants P534-1–P534-8 crosslinked; AGENTS playbook present.
#
# Refs: skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md,
#       frontend-dapp/src/utils/pairCatalogRank.ts,
#       docs/frontend.md § Pair catalog rank
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
echo "  GitLab #534 — pair catalog rank + human quote volume"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: catalog rank + volume format + pickers" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/pairCatalogRank.test.ts \
    src/utils/__tests__/formatAmount.test.ts \
    src/utils/__tests__/pairSearchQuery.test.ts \
    src/utils/__tests__/tokenSearchQuery.test.ts \
    src/components/trade/__tests__/PairSearchSelect.issue534.test.tsx \
    src/components/trade/__tests__/PairSearchSelect.issue301.test.tsx \
    src/pages/TradePage.test.tsx'

run_step "code: PairSearchSelect uses catalog sort + formatQuoteVolume24h" \
  grep -qE 'sortPairInfosByCatalog' frontend-dapp/src/components/trade/PairSearchSelect.tsx && \
  grep -qE 'formatQuoteVolume24h' frontend-dapp/src/components/trade/PairSearchSelect.tsx && \
  grep -qE 'Test pairs' frontend-dapp/src/components/trade/PairSearchSelect.tsx && \
  bash -c '! grep -qE "formatNum\(opt.volumeQuote24h" frontend-dapp/src/components/trade/PairSearchSelect.tsx'

run_step "code: Trade auto-pick + Charts catalog rank" \
  grep -qE 'firstCatalogPairAddress' frontend-dapp/src/pages/TradePage.tsx && \
  grep -qE 'sortIndexerPairsByCatalog' frontend-dapp/src/pages/ChartsPage.tsx && \
  grep -qE 'formatQuoteVolume24h' frontend-dapp/src/pages/PoolPage.tsx

run_step "code: Swap token empty browse uses compareTokenCatalog" \
  grep -qE 'compareTokenCatalog' frontend-dapp/src/utils/tokenSearchQuery.ts

run_step "docs: frontend.md P534-1–P534-8" \
  grep -qE 'pair-catalog-rank' docs/frontend.md && \
  grep -qE '\*\*P534-1\*\*' docs/frontend.md && \
  grep -qE '\*\*P534-8\*\*' docs/frontend.md

run_step "skill: AGENTS_FRONTEND_PAIR_CATALOG_RANK" \
  grep -qE '\*\*P534-1' skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md && \
  grep -qE '\*\*P534-8' skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md && \
  grep -qE 'make verify-issue-534' skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md

run_step "crosslinks: token search + pair USD + UST1 AMM + AGENTS.md" \
  grep -qE 'AGENTS_FRONTEND_PAIR_CATALOG_RANK|#534' skills/AGENTS_FRONTEND_TOKEN_SEARCH.md && \
  grep -qE 'AGENTS_FRONTEND_PAIR_CATALOG_RANK|#534' skills/AGENTS_INDEXER_PAIR_PRICE_USD.md && \
  grep -qE 'AGENTS_FRONTEND_PAIR_CATALOG_RANK|#534' skills/AGENTS_UST1_SECONDARY_AMM.md && \
  grep -qE 'AGENTS_FRONTEND_PAIR_CATALOG_RANK|#534' AGENTS.md

run_step "indexer docs: volume_quote_24h stays raw" \
  grep -qE 'volume_quote_24h.*raw|#534' docs/indexer-invariants.md

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
