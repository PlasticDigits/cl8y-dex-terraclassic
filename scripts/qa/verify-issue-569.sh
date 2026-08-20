#!/usr/bin/env bash
# Verification for GitLab #569: /protocol total USD pair liquidity + 24h/30d % change.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env; no wasm deploy).
# Playwright e2e-smoke is optional when frontend-dapp/node_modules exists.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }

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
echo "  GitLab #569 — Protocol pool TVL + 24h/30d liquidity Δ%"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs: invariants + skills + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P569-1" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P569-8" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "Protocol pool TVL (#569)" docs/indexer-invariants.md
    grep -q "global_liquidity_snapshots" docs/runbooks/overview-global-stats-brin.md
    grep -q "AGENTS_FRONTEND_PROTOCOL_STATS" AGENTS.md
    grep -q "verify-issue-569" AGENTS.md
    grep -q "protocol-stat-liquidity" docs/frontend.md
    grep -q "total_liquidity_usd" docs/frontend.md
    grep -q "hub_prices.tvl_usd" skills/AGENTS_INDEXER_HUB_USD.md
    test -f indexer/migrations/20260820120000_global_stats_pool_tvl.sql
  '

run_step "source: overview additive TVL; GET does not scan snapshots; CG mislabel documented" \
  bash -c '
    set -euo pipefail
    grep -q total_liquidity_usd indexer/src/api/overview.rs
    grep -q liquidity_change_24h_pct indexer/src/api/overview.rs
    grep -q liquidity_change_30d_pct indexer/src/api/overview.rs
    grep -q refresh_protocol_liquidity indexer/src/db/queries/volume.rs
    grep -q protocol-stat-liquidity frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx
    grep -q formatProtocolPct frontend-dapp/src/utils/formatProtocolStats.ts
    grep -q "GitLab #569" indexer/src/api/cg.rs
    if grep -nF "liquidity_in_usd" frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx \
         frontend-dapp/src/pages/ProtocolPage.tsx 2>/dev/null; then
      echo "Protocol UI must not read CG liquidity_in_usd" >&2
      exit 1
    fi
  '

run_step "indexer lib: protocol_tvl math" \
  bash -c 'cd indexer && cargo test --lib protocol_tvl -- --quiet'

run_step "indexer integration: overview + protocol liquidity + global stats" \
  bash -c 'cd indexer && cargo test --test api_overview --test indexer_overview_global_stats --test indexer_protocol_liquidity -- --test-threads=1 --quiet'

run_step "frontend: Protocol RTL + formatProtocolPct" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/pages/ProtocolPage.test.tsx src/utils/__tests__/formatProtocolStats.test.ts -t "ProtocolPage|formatProtocolUsd|formatProtocolCount|formatProtocolPct|formatProtocolOracleUsd"'

if [ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]; then
  COMMON="$(git rev-parse --git-common-dir)"
  MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
  if [ -f "$MAIN_ROOT/frontend-dapp/.env.local" ] && [ "$MAIN_ROOT" != "$REPO_ROOT" ]; then
    cp "$MAIN_ROOT/frontend-dapp/.env.local" "$REPO_ROOT/frontend-dapp/.env.local"
    echo ""
    echo "[bootstrap] copied frontend-dapp/.env.local from primary checkout (gitignored)"
  fi
fi

if [[ "${VERIFY_ISSUE_569_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] skipped (VERIFY_ISSUE_569_SKIP_E2E=1)"
  ok "playwright e2e-smoke protocol-page (skipped)"
elif [ -d "$REPO_ROOT/frontend-dapp/node_modules/@playwright/test" ] || [ -d "$REPO_ROOT/frontend-dapp/node_modules/playwright" ]; then
  run_step "playwright e2e-smoke protocol-page (5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30569 PLAYWRIGHT_BASE_URL=http://127.0.0.1:30569 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/protocol-page.spec.ts'
else
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] SKIP (no Playwright install)"
  ok "playwright e2e-smoke protocol-page (skipped — no node_modules playwright)"
fi

if [[ "${VERIFY_ISSUE_569_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related 550/556] skipped (VERIFY_ISSUE_569_SKIP_RELATED=1)"
  ok "related verifies (skipped)"
else
  run_step "related: verify-issue-550" \
    bash -c 'VERIFY_ISSUE_550_SKIP_E2E=1 make verify-issue-550'
  run_step "related: verify-issue-556" \
    bash -c 'VERIFY_ISSUE_556_SKIP_E2E=1 VERIFY_ISSUE_556_SKIP_RELATED=1 make verify-issue-556'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
