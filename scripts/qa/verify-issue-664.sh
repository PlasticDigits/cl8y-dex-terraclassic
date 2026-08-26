#!/usr/bin/env bash
# Automated verification for GitLab #664 — /trade + /charts v2 LP USD identity.
#
# Proves:
#   1. GET /api/v1/pairs/{addr} stamp / omit / 404; GET is not a live TVL sum.
#   2. PairTokenLinks chip present / omitted / hostile; Trade + Charts mounts.
#   3. /pool identity does not take liquidityUsd; Trade does not getPool.
#   4. Docs/skills/invariants T664 + AGENTS playbook.
#   5. Playwright smoke (5 workers, no e2e-tx).
#
# Refs: skills/AGENTS_FRONTEND_TRADE_IDENTITY_LP.md,
#       indexer/src/db/queries/pair_liquidity_usd.rs,
#       docs/indexer-invariants.md § Single-pair v2 LP USD
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
echo "  GitLab #664 — Trade / Charts identity v2 LP USD"
echo "════════════════════════════════════════════════════════════════"

run_step "docs: invariants + skills + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_TRADE_IDENTITY_LP.md
    grep -q "T664-1" skills/AGENTS_FRONTEND_TRADE_IDENTITY_LP.md
    grep -q "T664-8" skills/AGENTS_FRONTEND_TRADE_IDENTITY_LP.md
    grep -q "make verify-issue-664" skills/AGENTS_FRONTEND_TRADE_IDENTITY_LP.md
    grep -q "Single-pair v2 LP USD (#664)" docs/indexer-invariants.md
    grep -q "pair_liquidity_usd" docs/indexer-invariants.md
    grep -q "T664-1" docs/frontend.md
    grep -q "token-identity-v2-lp-usd" docs/frontend.md
    grep -q "AGENTS_FRONTEND_TRADE_IDENTITY_LP" AGENTS.md
    grep -q "verify-issue-664" AGENTS.md
    grep -q "verify-issue-664" Makefile
    grep -q "#664" skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md
    grep -q "#664" skills/AGENTS_FRONTEND_POOL_TABLE.md
    grep -q "#664" skills/AGENTS_FRONTEND_CHARTS_PAIR_STATS.md
    grep -q "#664" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "#664" skills/AGENTS_INDEXER_HUB_USD.md
    grep -q "pair_liquidity_usd" docs/runbooks/overview-global-stats-brin.md
    test -f indexer/migrations/20260826150000_pair_liquidity_usd.sql
  '

run_step "code: stamp JOIN on GET; no live reserves in get_pair; list does not emit yet" \
  bash -c '
    set -euo pipefail
    grep -q "liquidity_usd" indexer/src/api/pairs.rs
    grep -q "get_pair_liquidity_usd" indexer/src/api/pairs.rs
    grep -q "replace_pair_liquidity_usd" indexer/src/indexer/protocol_tvl.rs
    grep -q "collect_priced_pair_tvls" indexer/src/indexer/protocol_tvl.rs
    python3 - <<'"'"'PY'"'"'
from pathlib import Path
text = Path("indexer/src/api/pairs.rs").read_text()
# Isolate get_pair handler body until the next pub async fn.
start = text.find("pub async fn get_pair(")
end = text.find("pub async fn get_pair_candles")
chunk = text[start:end]
if "pair_reserves" in chunk or "protocol_pair_tvl" in chunk or "list_reserve_pairs" in chunk:
    raise SystemExit("get_pair must not compute live TVL / join pair_reserves")
if "get_pair_liquidity_usd" not in chunk:
    raise SystemExit("get_pair must read pair_liquidity_usd stamp")
PY
    grep -q "liquidityUsd" frontend-dapp/src/pages/TradePage.tsx
    grep -q "liquidityUsd" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "token-identity-v2-lp-usd" frontend-dapp/src/components/ui/PairTokenLinks.tsx
    grep -q "formatPairV2LpUsd" frontend-dapp/src/utils/formatProtocolStats.ts
    grep -q "liquidity_usd?: string | null" frontend-dapp/src/types/index.ts
    if grep -nE "liquidityUsd" frontend-dapp/src/components/pool/PoolPairsTable.tsx; then
      echo "/pool table must not pass liquidityUsd (#655 owns the column)" >&2
      exit 1
    fi
    if grep -nE "getPool" frontend-dapp/src/pages/TradePage.tsx; then
      echo "Trade identity must not LCD getPool" >&2
      exit 1
    fi
    if grep -nE "getOverview" frontend-dapp/src/pages/TradePage.tsx; then
      echo "Trade must not fetch overview as pair TVL" >&2
      exit 1
    fi
    if grep -qE "liquidity_usd" indexer/src/api/pairs.rs && \
       grep -n "liquidity_usd:" indexer/src/api/pairs.rs | grep -q "None"; then
      :
    else
      echo "list/token PairResponse should leave liquidity_usd None until #655" >&2
    fi
  '

run_step "indexer lib: protocol_tvl collect still matches census math" \
  bash -c 'cd indexer && cargo test --lib protocol_tvl -- --quiet'

run_step "indexer integration: pair GET stamp + protocol liquidity refresh" \
  bash -c '
    set -euo pipefail
    if [ ! -f indexer/.env ]; then
      echo "indexer/.env missing — running make setup-indexer-postgres…"
      make setup-indexer-postgres
    fi
    export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
    cd indexer && cargo test --test api_pairs --test indexer_protocol_liquidity -- --test-threads=1 --quiet
  '

run_step "frontend: formatPairV2LpUsd + PairTokenLinks + Trade/Charts #664" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/formatProtocolStats.test.ts \
    src/components/ui/__tests__/PairTokenLinks.test.tsx \
    src/pages/TradePage.test.tsx \
    src/pages/ChartsPage.test.tsx \
    -t "664"'

run_step "Playwright smoke E1–E3 (5 workers, no e2e-tx / no globalSetup seed)" \
  bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30664 bash scripts/with-node.sh --cwd frontend-dapp -- \
    ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 \
    e2e/trade-identity-lp-664.spec.ts'

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
