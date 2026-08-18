#!/usr/bin/env bash
# Verification for GitLab #556: DEX hub USD for cUSTC/UST1/USTR + Protocol DEX card.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env; no wasm deploy).
# Playwright e2e-smoke is optional when frontend-dapp/node_modules exists.
# Related ladders (515/550/522/543/524) run unless VERIFY_ISSUE_556_SKIP_RELATED=1.
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
echo "  GitLab #556 — DEX hub USD (cUSTC / UST1 / USTR)"
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
    test -f skills/AGENTS_INDEXER_HUB_USD.md
    grep -q "DEX hub USD (#556)" docs/indexer-invariants.md
    grep -q "H1" skills/AGENTS_INDEXER_HUB_USD.md
    grep -q "H10" skills/AGENTS_INDEXER_HUB_USD.md
    grep -q "AGENTS_INDEXER_HUB_USD" AGENTS.md
    grep -q "verify-issue-556" AGENTS.md
    grep -q "protocol-dex-hub-prices" docs/frontend.md
    grep -q "GET /api/v1/hub-prices" docs/runbooks/indexer-external-oracle.md
    grep -q "hub_prices" skills/AGENTS_INDEXER_PAIR_PRICE_USD.md
    grep -q "protocol-dex-hub-prices" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "HUB_CUSTC_ADDRESS" indexer/.env.example
    test -f indexer/migrations/20260818180000_hub_prices.sql
  '

run_step "source: ingest has no USTR_PER_USTC; CEX ALL len 3; CG last_price human" \
  bash -c '
    set -euo pipefail
    if grep -nF "USTR_PER_USTC" indexer/src/indexer/pair_price_usd.rs; then
      echo "pair_price_usd.rs must not use USTR_PER_USTC" >&2
      exit 1
    fi
    if grep -nE "QuoteUsdKind::Peg1 => Some\\(BigDecimal::from\\(1\\)\\)" indexer/src/indexer/pair_price_usd.rs; then
      echo "Peg1 must not hardcode \$1" >&2
      exit 1
    fi
    grep -qF "ALL: [OracleTicker; 3]" indexer/src/indexer/oracle.rs
    grep -q OracleTicker::Vfdusd indexer/src/indexer/oracle.rs
    ! grep -q "OracleTicker::Ustr" indexer/src/indexer/oracle.rs
    grep -q "hub-prices" indexer/src/api/mod.rs
    grep -q ProtocolDexHubPrices frontend-dapp/src/pages/ProtocolPage.tsx
    grep -q formatPairPrice frontend-dapp/src/components/protocol/ProtocolDexHubPrices.tsx
    if grep -nF "formatNum" frontend-dapp/src/components/protocol/ProtocolDexHubPrices.tsx; then
      echo "hub USD must use formatPairPrice, not formatNum" >&2
      exit 1
    fi
    grep -q "close_price" indexer/src/api/cg.rs
    if grep -n "last_price: stats" -A2 indexer/src/api/cg.rs | grep -q close_price_usd; then
      echo "CG last_price must stay human close_price (H10)" >&2
      exit 1
    fi
    grep -qF "UST1_LP_USTR_PER_USTC" scripts/rebalance-mint-ust1-lp.sh
    grep -qF "USTR_PER_USTC" scripts/lib/ust1-lp-rebalance-math.py
  '

run_step "indexer lib: hub_usd + pair_price_usd" \
  bash -c 'cd indexer && cargo test --lib --quiet hub_usd && cargo test --lib --quiet pair_price'

run_step "indexer integration: hub-prices API + ingest + oracle 400s" \
  bash -c 'cd indexer && cargo test --test api_hub_prices --test volume_usd_catalog --test swap_price_human_usd --test api_oracle --test api_overview -- --test-threads=1 --quiet'

run_step "frontend: Protocol hub card + pairPriceUsd + ticker allowlist" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/pages/ProtocolPage.test.tsx src/utils/__tests__/hubPriceTicker.test.ts src/utils/__tests__/pairPriceUsd.test.ts src/services/indexer/__tests__/client.test.ts -t "ProtocolPage|parseHubPriceTicker|resolveDisplayTapeLastPriceUsd|hub price ticker|allowlists ticker"'

if [ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]; then
  COMMON="$(git rev-parse --git-common-dir)"
  MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
  if [ -f "$MAIN_ROOT/frontend-dapp/.env.local" ] && [ "$MAIN_ROOT" != "$REPO_ROOT" ]; then
    cp "$MAIN_ROOT/frontend-dapp/.env.local" "$REPO_ROOT/frontend-dapp/.env.local"
    echo ""
    echo "[bootstrap] copied frontend-dapp/.env.local from primary checkout (gitignored)"
  fi
fi

if [[ "${VERIFY_ISSUE_556_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] skipped (VERIFY_ISSUE_556_SKIP_E2E=1)"
  ok "playwright e2e-smoke protocol-page (skipped)"
elif [ -d "$REPO_ROOT/frontend-dapp/node_modules/@playwright/test" ] || [ -d "$REPO_ROOT/frontend-dapp/node_modules/playwright" ]; then
  run_step "playwright e2e-smoke protocol-page (5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30556 PLAYWRIGHT_BASE_URL=http://127.0.0.1:30556 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/protocol-page.spec.ts'
else
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] SKIP (no Playwright install)"
  ok "playwright e2e-smoke protocol-page (skipped — no node_modules playwright)"
fi

if [[ "${VERIFY_ISSUE_556_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related 515/550/522/543/524] skipped (VERIFY_ISSUE_556_SKIP_RELATED=1)"
  ok "related verifies (skipped)"
else
  run_step "related: verify-issue-515" make verify-issue-515
  run_step "related: verify-issue-550" make verify-issue-550
  run_step "related: verify-issue-522" make verify-issue-522
  run_step "related: verify-issue-543" make verify-issue-543
  run_step "related: verify-issue-524" make verify-issue-524
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
