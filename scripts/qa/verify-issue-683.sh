#!/usr/bin/env bash
# Verification for GitLab #683: /protocol fee USD for CL8Y + factory-listed economic tokens.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env; no wasm deploy).
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
echo "  GitLab #683 — factory economic fee USD (CL8Y + listed non-gems)"
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
    test -f skills/AGENTS_INDEXER_ECONOMIC_FEE_USD.md
    grep -q "EFee-1" skills/AGENTS_INDEXER_ECONOMIC_FEE_USD.md
    grep -q "EFee-8" skills/AGENTS_INDEXER_ECONOMIC_FEE_USD.md
    grep -q "Economic fee USD (#683)" docs/indexer-invariants.md
    grep -q "economic_token_marks" docs/indexer-invariants.md
    grep -q "NULL-only" docs/runbooks/overview-global-stats-brin.md
    grep -q "AGENTS_INDEXER_ECONOMIC_FEE_USD" AGENTS.md
    grep -q "verify-issue-683" AGENTS.md
    grep -q "factory economic marks" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "HUB_CL8Y_ADDRESS" skills/AGENTS_INDEXER_HUB_USD.md
    grep -q "economic_token_marks" skills/AGENTS_INDEXER_HUB_USD.md
    test -f indexer/migrations/20260827140000_economic_token_marks.sql
    grep -q "HUB_CL8Y_ADDRESS" indexer/src/config.rs
    grep -q "DEFAULT_HUB_CL8Y_ADDRESS" indexer/src/config.rs
  '

run_step "source: GET does not scan events; hub allowlist unchanged; no CL8Y in quote_usd_kind" \
  bash -c '
    set -euo pipefail
    grep -q "quote_usd_kind(\"CL8Y\", None), None" indexer/src/indexer/pair_price_usd.rs
    grep -q "fn fee_usd_per_human" indexer/src/indexer/protocol_fees.rs
    grep -q "fn resolve_economic_marks" indexer/src/indexer/economic_usd.rs
    grep -q "fn backfill_null_fee_usd" indexer/src/db/queries/protocol_fees.rs
    grep -q "economic_token_marks" indexer/src/db/queries/hub_prices.rs
    grep -q "CL8Y-cb" frontend-dapp/src/pages/ProtocolPage.test.tsx
    if grep -nE "SUM\\(.*protocol_fee_events|FROM protocol_fee_events" \
         indexer/src/api/overview.rs indexer/src/api/protocol_fees.rs 2>/dev/null; then
      echo "GET handlers must not SUM protocol_fee_events" >&2
      exit 1
    fi
    if grep -nE "HUB_TICKERS: \\[.*, \"cl8y\"" indexer/src/indexer/hub_usd.rs; then
      echo "Hub ticker allowlist must stay four cells" >&2
      exit 1
    fi
    grep -q "HUB_TICKERS: \\[&str; 4\\] = \\[\"custc\", \"lunc\", \"ust1\", \"ustr\"\\]" indexer/src/indexer/hub_usd.rs
  '

run_step "indexer lib: economic walker + fee USD + hub allowlist" \
  bash -c 'cd indexer && cargo test --lib --quiet -- economic_usd fee_usd ticker_allowlist'

run_step "indexer integration: economic fee USD + NULL-only backfill + hub GET" \
  bash -c 'cd indexer && cargo test --test indexer_economic_fee_usd --test api_hub_prices -- --test-threads=1 --quiet'

if [[ ! -x "$REPO_ROOT/frontend-dapp/node_modules/.bin/vitest" ]]; then
  SIBLING="$(dirname "$REPO_ROOT")/cl8y-dex-terraclassic/frontend-dapp/node_modules"
  if [[ -x "$SIBLING/.bin/vitest" ]]; then
    ln -sfn "$SIBLING" "$REPO_ROOT/frontend-dapp/node_modules"
    echo "[bootstrap] linked frontend-dapp/node_modules from primary checkout"
  else
    echo "[bootstrap] frontend-dapp/node_modules missing — npm ci…"
    bash "$REPO_ROOT/scripts/with-node.sh" --cwd frontend-dapp -- npm ci
  fi
fi

run_step "frontend: Protocol CL8Y-cb token USD + XSS text" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/pages/ProtocolPage.test.tsx'

if [[ "${VERIFY_ISSUE_683_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related 586/613/614/556/569/631] skipped (VERIFY_ISSUE_683_SKIP_RELATED=1)"
  ok "related verifies (skipped)"
else
  run_step "related: verify-issue-586" \
    bash -c 'VERIFY_ISSUE_586_SKIP_E2E=1 VERIFY_ISSUE_586_SKIP_RELATED=1 make verify-issue-586'
  run_step "related: verify-issue-613" \
    bash -c 'VERIFY_ISSUE_613_SKIP_586=1 make verify-issue-613'
  run_step "related: verify-issue-614" \
    bash -c 'VERIFY_ISSUE_614_SKIP_E2E=1 VERIFY_ISSUE_614_SKIP_RELATED=1 make verify-issue-614'
  run_step "related: verify-issue-556" \
    bash -c 'VERIFY_ISSUE_556_SKIP_E2E=1 VERIFY_ISSUE_556_SKIP_RELATED=1 make verify-issue-556'
  run_step "related: verify-issue-569" \
    bash -c 'VERIFY_ISSUE_569_SKIP_E2E=1 VERIFY_ISSUE_569_SKIP_RELATED=1 make verify-issue-569'
  run_step "related: verify-issue-631" \
    bash -c 'VERIFY_ISSUE_631_SKIP_RELATED=1 make verify-issue-631'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
