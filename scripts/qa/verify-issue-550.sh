#!/usr/bin/env bash
# Verification for GitLab #550: /protocol global USD stats + unified USTC/LUNC/vFDUSD oracle.
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
echo "  GitLab #550 — Protocol global USD stats + unified oracle"
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
    test -f skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md
    grep -q "P550-1" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "vfdusd" skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md
    grep -q "vfdusd" docs/runbooks/indexer-external-oracle.md
    grep -q "active_pairs_24h" docs/runbooks/overview-global-stats-brin.md
    grep -q "Protocol global stats (#550)" docs/indexer-invariants.md
    grep -q "AGENTS_FRONTEND_PROTOCOL_STATS" AGENTS.md
    grep -q "verify-issue-550" AGENTS.md
    grep -q "protocol-global-stats" docs/frontend.md
    grep -q "vfdusd" docs/frontend.md
    test -f indexer/migrations/20260818120000_global_stats_windows_and_census.sql
  '

run_step "source: Protocol calls getOraclePrice with ticker; one oracle card; ALL len 3" \
  bash -c '
    set -euo pipefail
    if grep -nF "getOraclePrice()" frontend-dapp/src/pages/ProtocolPage.tsx \
         frontend-dapp/src/components/protocol/*.tsx \
         frontend-dapp/src/components/protocol/*.ts 2>/dev/null; then
      echo "Protocol still calls getOraclePrice() without a ticker" >&2
      exit 1
    fi
    grep -qF "getOraclePrice(ticker)" frontend-dapp/src/components/protocol/useProtocolOracleQueries.ts
    grep -q ProtocolOracleCard frontend-dapp/src/pages/ProtocolPage.tsx
    grep -q ProtocolGlobalStats frontend-dapp/src/pages/ProtocolPage.tsx
    ! grep -q "Recent USTC/USD history" frontend-dapp/src/pages/ProtocolPage.tsx
    grep -q protocol-oracle frontend-dapp/src/components/protocol/ProtocolOracleCard.tsx
    grep -q protocol-global-stats frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx
    grep -q OracleTicker::Vfdusd indexer/src/indexer/oracle.rs
    grep -qF "ALL: [OracleTicker; 3]" indexer/src/indexer/oracle.rs
    grep -q first-digital-usd indexer/src/indexer/oracle.rs
    grep -q FDUSDUSDT indexer/src/indexer/oracle.rs
    grep -q total_volume_7d_usd indexer/src/api/overview.rs
    grep -q tokens_added_30d indexer/src/api/overview.rs
    grep -q active_pairs_24h indexer/src/api/overview.rs
    grep -q count_assets indexer/src/api/overview.rs
    grep -q "path: '\''/protocol'\''" frontend-dapp/e2e/design-tokens-visual.spec.ts
    if grep -n get_all_assets indexer/src/api/overview.rs; then
      echo "token_count must not use get_all_assets().len()" >&2
      exit 1
    fi
  '

run_step "indexer lib: oracle ticker symbols + vfdusd" \
  bash -c 'cd indexer && cargo test --lib oracle -- --quiet'

run_step "indexer integration: api_oracle + api_overview + global stats" \
  bash -c 'cd indexer && cargo test --test api_oracle --test api_overview --test indexer_overview_global_stats -- --test-threads=1 --quiet'

run_step "frontend: Protocol RTL + ticker allowlist + client" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/pages/ProtocolPage.test.tsx src/utils/__tests__/protocolOracleTicker.test.ts src/utils/__tests__/formatProtocolStats.test.ts src/services/indexer/__tests__/client.test.ts -t "allowlists ticker|ProtocolPage|parseProtocolOracleTicker|formatProtocolUsd|formatProtocolCount|formatProtocolOracleUsd"'

# Worktrees do not inherit gitignored frontend-dapp/.env.local; copy from the primary
# checkout when present so P1 can assert factory/router without a full redeploy.
if [ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]; then
  COMMON="$(git rev-parse --git-common-dir)"
  MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
  if [ -f "$MAIN_ROOT/frontend-dapp/.env.local" ] && [ "$MAIN_ROOT" != "$REPO_ROOT" ]; then
    cp "$MAIN_ROOT/frontend-dapp/.env.local" "$REPO_ROOT/frontend-dapp/.env.local"
    echo ""
    echo "[bootstrap] copied frontend-dapp/.env.local from primary checkout (gitignored)"
  fi
fi

if [[ "${VERIFY_ISSUE_550_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] skipped (VERIFY_ISSUE_550_SKIP_E2E=1)"
  ok "playwright e2e-smoke protocol-page (skipped)"
elif [ -d "$REPO_ROOT/frontend-dapp/node_modules/@playwright/test" ] || [ -d "$REPO_ROOT/frontend-dapp/node_modules/playwright" ]; then
  run_step "playwright e2e-smoke protocol-page (5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30550 PLAYWRIGHT_BASE_URL=http://127.0.0.1:30550 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/protocol-page.spec.ts'
else
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] SKIP (no Playwright install)"
  ok "playwright e2e-smoke protocol-page (skipped — no node_modules playwright)"
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
