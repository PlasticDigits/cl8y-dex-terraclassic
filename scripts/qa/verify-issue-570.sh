#!/usr/bin/env bash
# Verification for GitLab #570: Protocol DEX hub cUSTC wrap link + LUNC/USD column.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env; no wasm deploy).
# Playwright e2e-smoke is optional when frontend-dapp/node_modules exists.
# Related ladders (556/550/515/541) run unless VERIFY_ISSUE_570_SKIP_RELATED=1.
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
echo "  GitLab #570 — Protocol DEX hub wrap identity + LUNC column"
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
    test -f skills/AGENTS_FRONTEND_PROTOCOL_HUB.md
    grep -q "H11" skills/AGENTS_FRONTEND_PROTOCOL_HUB.md
    grep -q "H16" skills/AGENTS_FRONTEND_PROTOCOL_HUB.md
    grep -q "H11" skills/AGENTS_INDEXER_HUB_USD.md
    grep -q "verify-issue-570" AGENTS.md
    grep -q "AGENTS_FRONTEND_PROTOCOL_HUB" AGENTS.md
    grep -q "DEX hub wrap identity (#570)" docs/indexer-invariants.md
    grep -q "protocol-dex-hub-lunc" docs/frontend.md
    grep -q "HUB_CLUNC_ADDRESS" indexer/.env.example
    grep -q "HUB_CLUNC_ADDRESS" skills/AGENTS_INDEXER_HUB_USD.md
    grep -q "data-testid={\`protocol-dex-hub-\${ticker}-token\`}" frontend-dapp/src/components/protocol/ProtocolDexHubPrices.tsx
    grep -q "protocol-dex-hub-lunc-token" docs/frontend.md
    test -f indexer/migrations/20260820100000_hub_prices_lunc.sql
    grep -q "lunc" indexer/migrations/20260820100000_hub_prices_lunc.sql
  '

run_step "source: LUNC hub is oracle not pool; CEX tabs stay 3; no uluna Finder" \
  bash -c '
    set -euo pipefail
    grep -qF "resolve_lunc_hub_mark" indexer/src/indexer/hub_usd.rs
    grep -qF "HubTicker::Lunc" indexer/src/indexer/hub_usd.rs
    grep -qF "asset_address" indexer/src/api/hub_prices.rs
    grep -q "HUB_TICKERS: \\[&str; 4\\]" indexer/src/indexer/hub_usd.rs
    grep -qF "ALL: [OracleTicker; 3]" indexer/src/indexer/oracle.rs
    ! grep -q "OracleTicker::Ustr" indexer/src/indexer/oracle.rs
    grep -q "sm:grid-cols-2 xl:grid-cols-4" frontend-dapp/src/components/protocol/ProtocolDexHubPrices.tsx
    if grep -nF "formatNum" frontend-dapp/src/components/protocol/ProtocolDexHubPrices.tsx; then
      echo "hub USD must use formatPairPrice, not formatNum" >&2
      exit 1
    fi
    if grep -nE "getExplorerAddressUrl\\(['\''\"]uluna" frontend-dapp/src/components/protocol/ProtocolDexHubPrices.tsx; then
      echo "must not fabricate Finder URL for native uluna" >&2
      exit 1
    fi
    grep -q "resolveHubOracleWrapAddress" frontend-dapp/src/components/protocol/ProtocolDexHubPrices.tsx
    grep -q "getExplorerAddressUrl" frontend-dapp/src/utils/hubOracleWrapAddress.ts
  '

run_step "indexer lib: hub_usd lunc + wrap sanitize" \
  bash -c 'cd indexer && cargo test --lib --quiet hub_usd'

run_step "indexer integration: hub-prices LUNC + 400s + oracle catalog" \
  bash -c 'cd indexer && cargo test --test api_hub_prices --test api_oracle -- --test-threads=1 --quiet'

run_step "frontend: Protocol hub wrap + LUNC column + ticker allowlist" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/ProtocolPage.test.tsx \
    src/utils/__tests__/hubPriceTicker.test.ts \
    src/utils/__tests__/hubOracleWrapAddress.test.ts \
    src/services/indexer/__tests__/client.test.ts \
    -t "ProtocolPage|parseHubPriceTicker|resolveHubOracleWrapAddress|hub price ticker"'

if [ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]; then
  COMMON="$(git rev-parse --git-common-dir)"
  MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
  if [ -f "$MAIN_ROOT/frontend-dapp/.env.local" ] && [ "$MAIN_ROOT" != "$REPO_ROOT" ]; then
    cp "$MAIN_ROOT/frontend-dapp/.env.local" "$REPO_ROOT/frontend-dapp/.env.local"
    echo ""
    echo "[bootstrap] copied frontend-dapp/.env.local from primary checkout (gitignored)"
  fi
fi

if [[ "${VERIFY_ISSUE_570_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] skipped (VERIFY_ISSUE_570_SKIP_E2E=1)"
  ok "playwright e2e-smoke protocol-page (skipped)"
elif [ -d "$REPO_ROOT/frontend-dapp/node_modules/@playwright/test" ] || [ -d "$REPO_ROOT/frontend-dapp/node_modules/playwright" ]; then
  run_step "playwright e2e-smoke protocol-page (5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30570 PLAYWRIGHT_BASE_URL=http://127.0.0.1:30570 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/protocol-page.spec.ts'
else
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] SKIP (no Playwright install)"
  ok "playwright e2e-smoke protocol-page (skipped — no node_modules playwright)"
fi

if [[ "${VERIFY_ISSUE_570_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related 556/550/515/541] skipped (VERIFY_ISSUE_570_SKIP_RELATED=1)"
  ok "related verifies (skipped)"
else
  run_step "related: verify-issue-556" env VERIFY_ISSUE_556_SKIP_RELATED=1 make verify-issue-556
  run_step "related: verify-issue-550" make verify-issue-550
  run_step "related: verify-issue-515" make verify-issue-515
  run_step "related: verify-issue-541" make verify-issue-541
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
