#!/usr/bin/env bash
# Verification for GitLab #652: /protocol inline Δ%, volume prior-window %, UTC-day series.
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
echo "  GitLab #652 — Protocol inline Δ% + volume % + UTC-day series"
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
    grep -q "P652-1" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P652-7" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "Protocol volume Δ% + daily (#652)" docs/indexer-invariants.md
    grep -q "protocol_daily_volume" docs/indexer-invariants.md
    grep -q "volume_change" docs/runbooks/overview-global-stats-brin.md
    grep -q "verify-issue-652" AGENTS.md
    grep -q "protocol-volume-daily-chart" docs/frontend.md
    grep -q "P652-1–P652-7" docs/frontend.md
    test -f indexer/migrations/20260826120000_protocol_volume_change_and_daily.sql
    grep -q "protocol/volume/daily" indexer/src/api/mod.rs
  '

run_step "source: additive volume Δ%; daily allowlist; no Llama N+1; no GET swap_events scan" \
  bash -c '
    set -euo pipefail
    grep -q volume_change_24h_pct indexer/src/api/overview.rs
    grep -q flow_change_pct indexer/src/db/queries/volume.rs
    grep -q refresh_protocol_daily indexer/src/indexer/volume_aggregator.rs
    grep -q protocol-volume-daily-chart frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx
    grep -q getProtocolVolumeDaily frontend-dapp/src/services/indexer/client.ts
    grep -q "variant=\"flat\"" frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx
    grep -q "variant=\"flat\"" frontend-dapp/src/components/protocol/ProtocolFeeStats.tsx
    if grep -nE "getDefillamaDaily|/defillama/daily" \
         frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx \
         frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx \
         frontend-dapp/src/pages/ProtocolPage.tsx 2>/dev/null; then
      echo "Protocol must not call GET /defillama/daily" >&2
      exit 1
    fi
    if grep -nE "FROM swap_events|SUM\\(.*swap_events" \
         indexer/src/api/overview.rs indexer/src/api/protocol_volume.rs 2>/dev/null; then
      echo "GET handlers must not SUM swap_events" >&2
      exit 1
    fi
    if grep -nE "from|to" indexer/src/api/defillama.rs | grep -qi "Query"; then
      echo "Do not add from/to to Llama in this ticket" >&2
      exit 1
    fi
  '

run_step "indexer lib: protocol_volume days allowlist + flow_change_pct" \
  bash -c 'cd indexer && cargo test --lib protocol_volume -- --quiet && cargo test --lib protocol_fees -- flow_change_pct --quiet'

run_step "indexer integration: volume Δ% + daily + overview keys" \
  bash -c 'cd indexer && cargo test --test indexer_protocol_volume --test api_overview -- --test-threads=1 --quiet'

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

run_step "frontend: Protocol RTL + StatBox + trailing-window copy" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/pages/ProtocolPage.test.tsx src/components/ui/__tests__/StatBox.test.tsx src/utils/__tests__/trailingWindowCopy.test.ts src/utils/__tests__/formatProtocolStats.test.ts'

if [ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]; then
  COMMON="$(git rev-parse --git-common-dir)"
  MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
  if [ -f "$MAIN_ROOT/frontend-dapp/.env.local" ] && [ "$MAIN_ROOT" != "$REPO_ROOT" ]; then
    cp "$MAIN_ROOT/frontend-dapp/.env.local" "$REPO_ROOT/frontend-dapp/.env.local"
    echo ""
    echo "[bootstrap] copied frontend-dapp/.env.local from primary checkout (gitignored)"
  fi
fi

if [[ "${VERIFY_ISSUE_652_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] skipped (VERIFY_ISSUE_652_SKIP_E2E=1)"
  ok "playwright e2e-smoke protocol-page (skipped)"
elif [ -d "$REPO_ROOT/frontend-dapp/node_modules/@playwright/test" ] || [ -d "$REPO_ROOT/frontend-dapp/node_modules/playwright" ]; then
  run_step "playwright e2e-smoke protocol-page (5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30652 PLAYWRIGHT_BASE_URL=http://127.0.0.1:30652 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/protocol-page.spec.ts'
else
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] SKIP (no Playwright install)"
  ok "playwright e2e-smoke protocol-page (skipped — no node_modules playwright)"
fi

if [[ "${VERIFY_ISSUE_652_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related 550/569/586/576] skipped (VERIFY_ISSUE_652_SKIP_RELATED=1)"
  ok "related verifies (skipped)"
else
  run_step "related: verify-issue-550" \
    bash -c 'VERIFY_ISSUE_550_SKIP_E2E=1 VERIFY_ISSUE_550_SKIP_RELATED=1 make verify-issue-550'
  run_step "related: verify-issue-586" \
    bash -c 'VERIFY_ISSUE_586_SKIP_E2E=1 VERIFY_ISSUE_586_SKIP_RELATED=1 make verify-issue-586'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
