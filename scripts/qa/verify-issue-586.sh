#!/usr/bin/env bash
# Verification for GitLab #586: /protocol treasury fee tracking (24h/7d/30d USD, Δ%, source/token mix).
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
echo "  GitLab #586 — Protocol treasury fees 24h/7d/30d + source/token"
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
    grep -q "PFee-1" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "PFee-12" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "Protocol fees (#586)" docs/indexer-invariants.md
    grep -q "Protocol fees (GitLab #586)" docs/runbooks/overview-global-stats-brin.md
    grep -q "protocol_fee_events" docs/runbooks/overview-global-stats-brin.md
    grep -q "AGENTS_FRONTEND_PROTOCOL_STATS" AGENTS.md
    grep -q "verify-issue-586" AGENTS.md
    grep -q "protocol-fee-stats" docs/frontend.md
    grep -q "PFee-1–PFee-12" docs/frontend.md
    grep -q "WRAP_MAPPER_ADDRESS" skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md
    test -f indexer/migrations/20260821120000_protocol_fees.sql
  '

run_step "source: overview additive fees; GET does not scan events; no traders.total_fees_paid headline" \
  bash -c '
    set -euo pipefail
    grep -q total_fees_24h_usd indexer/src/api/overview.rs
    grep -q fees_change_24h_pct indexer/src/api/overview.rs
    grep -q refresh_protocol_fee_stats indexer/src/db/queries/volume.rs
    grep -q protocol-fee-stats frontend-dapp/src/components/protocol/ProtocolFeeStats.tsx
    grep -q getProtocolFees frontend-dapp/src/services/indexer/client.ts
    grep -q WRAP_MAPPER_ADDRESS indexer/src/config.rs
    if grep -nF "total_fees_paid" frontend-dapp/src/components/protocol/ProtocolFeeStats.tsx \
         frontend-dapp/src/pages/ProtocolPage.tsx 2>/dev/null; then
      echo "Protocol fee UI must not headline traders.total_fees_paid" >&2
      exit 1
    fi
    if grep -nE "SUM\\(.*protocol_fee_events|FROM protocol_fee_events" \
         indexer/src/api/overview.rs indexer/src/api/protocol_fees.rs 2>/dev/null; then
      echo "GET handlers must not SUM protocol_fee_events" >&2
      exit 1
    fi
  '

run_step "indexer lib: protocol_fees math + wrap pin" \
  bash -c 'cd indexer && cargo test --lib protocol_fees -- --quiet'

run_step "indexer integration: protocol fees + overview keys" \
  bash -c 'cd indexer && cargo test --test indexer_protocol_fees --test api_overview -- --test-threads=1 --quiet'

run_step "frontend: Protocol RTL + trailing-window copy" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/pages/ProtocolPage.test.tsx src/utils/__tests__/trailingWindowCopy.test.ts'

if [ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]; then
  COMMON="$(git rev-parse --git-common-dir)"
  MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
  if [ -f "$MAIN_ROOT/frontend-dapp/.env.local" ] && [ "$MAIN_ROOT" != "$REPO_ROOT" ]; then
    cp "$MAIN_ROOT/frontend-dapp/.env.local" "$REPO_ROOT/frontend-dapp/.env.local"
    echo ""
    echo "[bootstrap] copied frontend-dapp/.env.local from primary checkout (gitignored)"
  fi
fi

if [[ "${VERIFY_ISSUE_586_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] skipped (VERIFY_ISSUE_586_SKIP_E2E=1)"
  ok "playwright e2e-smoke protocol-page (skipped)"
elif [ -d "$REPO_ROOT/frontend-dapp/node_modules/@playwright/test" ] || [ -d "$REPO_ROOT/frontend-dapp/node_modules/playwright" ]; then
  run_step "playwright e2e-smoke protocol-page (5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30586 PLAYWRIGHT_BASE_URL=http://127.0.0.1:30586 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/protocol-page.spec.ts'
else
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] SKIP (no Playwright install)"
  ok "playwright e2e-smoke protocol-page (skipped — no node_modules playwright)"
fi

if [[ "${VERIFY_ISSUE_586_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related 550/569/576/577] skipped (VERIFY_ISSUE_586_SKIP_RELATED=1)"
  ok "related verifies (skipped)"
else
  run_step "related: verify-issue-550" \
    bash -c 'VERIFY_ISSUE_550_SKIP_E2E=1 VERIFY_ISSUE_550_SKIP_RELATED=1 make verify-issue-550'
  run_step "related: verify-issue-569" \
    bash -c 'VERIFY_ISSUE_569_SKIP_E2E=1 VERIFY_ISSUE_569_SKIP_RELATED=1 make verify-issue-569'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
