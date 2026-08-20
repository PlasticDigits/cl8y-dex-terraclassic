#!/usr/bin/env bash
# Verification for GitLab #571: /protocol vFDUSD tab — FDUSD reference + Venus 1 vFDUSD Price.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env; no wasm deploy, no live BSC).
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
echo "  GitLab #571 — Protocol vFDUSD FDUSD reference + Venus redeem"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

# Worktree `.env` must override a parent-shell TEST_DATABASE_URL aimed at a shared DB.
if [ -f "$REPO_ROOT/indexer/.env" ]; then
  _tu=$(grep -E '^TEST_DATABASE_URL=' "$REPO_ROOT/indexer/.env" | tail -1 | sed 's/^TEST_DATABASE_URL=//')
  if [ -n "${_tu:-}" ]; then
    export TEST_DATABASE_URL="$_tu"
  fi
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

# Sibling worktrees may apply unmerged sqlx versions onto shared dex_indexer_test.
# Isolate #571 migrate so VersionMissing(other-issue) cannot fail this ladder.
export TEST_DATABASE_URL="${TEST_DATABASE_URL_571:-postgres://cl8y_legal:cl8y_legal@127.0.0.1:5432/dex_indexer_test_571}"
export TEST_DB_LOCK_FILE="${TEST_DB_LOCK_FILE:-/tmp/cl8y-dex-indexer-test-571.seed.lock}"
if command -v psql >/dev/null 2>&1; then
  PGPASSWORD="${PGPASSWORD:-cl8y_legal}" timeout 5s psql -h 127.0.0.1 -p 5432 -U cl8y_legal -d postgres \
    -tc "SELECT 1 FROM pg_database WHERE datname='dex_indexer_test_571'" 2>/dev/null | grep -q 1 || \
  PGPASSWORD="${PGPASSWORD:-cl8y_legal}" timeout 5s psql -h 127.0.0.1 -p 5432 -U cl8y_legal -d postgres \
    -c "CREATE DATABASE dex_indexer_test_571 OWNER cl8y_legal" >/dev/null 2>&1 || true
fi

run_step "docs: V571 invariants + skills + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_INDEXER_VENUS_VFDUSD.md
    grep -q "V571-1" skills/AGENTS_INDEXER_VENUS_VFDUSD.md
    grep -q "0xC4eF4229FEc74Ccfe17B2bdeF7715fAC740BA0ba" skills/AGENTS_INDEXER_VENUS_VFDUSD.md
    grep -q "FDUSD reference price" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "1 vFDUSD Price" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "FDUSD reference price" docs/frontend.md
    grep -q "1 vFDUSD Price" docs/frontend.md
    grep -q "Venus vFDUSD redeem (#571)" docs/indexer-invariants.md
    grep -q "venus_vfdusd.rs" docs/runbooks/indexer-external-oracle.md
    grep -q "AGENTS_INDEXER_VENUS_VFDUSD" AGENTS.md
    grep -q "verify-issue-571" AGENTS.md
    grep -q "0xbd6d894d" indexer/src/indexer/venus_vfdusd.rs
    grep -q "0x182df0cd" indexer/src/indexer/venus_vfdusd.rs
    grep -q "SELECTOR_EXCHANGE_RATE_CURRENT" indexer/src/indexer/venus_vfdusd.rs
    grep -q "eth_call" indexer/src/indexer/venus_vfdusd.rs
    if grep -nE "eth_send(Transaction|RawTransaction)" indexer/src/indexer/venus_vfdusd.rs | grep -v "Never\|never\|not"; then
      echo "Venus poller must not send a BSC transaction" >&2
      exit 1
    fi
    if grep -n "eth_call_failover(client, rpc_urls, vtoken, SELECTOR_EXCHANGE_RATE_STORED" indexer/src/indexer/venus_vfdusd.rs; then
      echo "live Core Pool vFDUSD no longer dispatches exchangeRateStored" >&2
      exit 1
    fi
    grep -q "exchangeRateCurrent" indexer/src/indexer/venus_vfdusd.rs
    grep -q "protocol-oracle-vfdusd-venus" frontend-dapp/src/components/protocol/ProtocolOracleCard.tsx
    grep -q "FDUSD reference price" frontend-dapp/src/components/protocol/ProtocolOracleCard.tsx
    grep -q "getOracleVenusVfdusd" frontend-dapp/src/services/indexer/client.ts
    grep -q "venus_vfdusd_rates" indexer/migrations/20260820200000_venus_vfdusd_rates.sql
    if grep -n "VITE_.*BSC\|VITE_.*VENUS" frontend-dapp/src frontend-dapp/.env* 2>/dev/null | grep -v test; then
      echo "BSC/Venus must not leak into Vite env" >&2
      exit 1
    fi
  '

run_step "source: no vFDUSD / USD heading; Venus isolated to vfdusd tab" \
  bash -c '
    set -euo pipefail
    if grep -n "vFDUSD / USD" frontend-dapp/src/components/protocol/ProtocolOracleCard.tsx \
         frontend-dapp/src/pages/ProtocolPage.tsx 2>/dev/null; then
      echo "Protocol card still titles CEX as vFDUSD / USD" >&2
      exit 1
    fi
    grep -q "enabled: ticker === '\''vfdusd'\''" frontend-dapp/src/components/protocol/useProtocolOracleQueries.ts
    grep -q "pathSegment('\''vfdusd'\'')" frontend-dapp/src/services/indexer/client.ts
    grep -q "VENUS_SOURCE" indexer/src/api/oracle.rs
    grep -q "price/{ticker}/venus" indexer/src/api/mod.rs
  '

run_step "indexer lib: Venus conversion + eth_call mock + oracle tickers" \
  bash -c 'cd indexer && cargo test --lib venus_vfdusd -- --quiet && cargo test --lib oracle -- --quiet'

run_step "indexer integration: api_oracle (catalog, 400 fdusd, Venus additive JSON)" \
  bash -c 'cd indexer && cargo test --test api_oracle -- --test-threads=1 --quiet'

run_step "frontend: Protocol RTL + formatter + ticker + client + CSP" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/pages/ProtocolPage.test.tsx src/utils/__tests__/protocolOracleTicker.test.ts src/utils/__tests__/formatProtocolStats.test.ts src/services/indexer/__tests__/client.test.ts src/utils/__tests__/viteCsp.test.ts'

if [ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]; then
  COMMON="$(git rev-parse --git-common-dir)"
  MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
  if [ -f "$MAIN_ROOT/frontend-dapp/.env.local" ] && [ "$MAIN_ROOT" != "$REPO_ROOT" ]; then
    cp "$MAIN_ROOT/frontend-dapp/.env.local" "$REPO_ROOT/frontend-dapp/.env.local"
    echo ""
    echo "[bootstrap] copied frontend-dapp/.env.local from primary checkout (gitignored)"
  fi
fi

if [[ "${VERIFY_ISSUE_571_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] skipped (VERIFY_ISSUE_571_SKIP_E2E=1)"
  ok "playwright e2e-smoke protocol-page (skipped)"
elif [ -d "$REPO_ROOT/frontend-dapp/node_modules/@playwright/test" ] || [ -d "$REPO_ROOT/frontend-dapp/node_modules/playwright" ]; then
  run_step "playwright e2e-smoke protocol-page (5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30571 PLAYWRIGHT_BASE_URL=http://127.0.0.1:30571 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/protocol-page.spec.ts'
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
