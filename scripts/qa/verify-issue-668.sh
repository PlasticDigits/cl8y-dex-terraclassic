#!/usr/bin/env bash
# Verification for GitLab #668: /protocol UTC volume chart — USD axis, tooltip,
# Hourly/Daily/Monthly grain, viewport-sized bar count.
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
echo "  GitLab #668 — Protocol UTC volume grain chart"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs: P668 invariants + skills + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P668-1" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P668-8" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P668-9" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "Protocol volume grain chart (#668)" docs/indexer-invariants.md
    grep -q "protocol_hourly_volume" docs/indexer-invariants.md
    grep -q "GitLab #668" docs/runbooks/overview-global-stats-brin.md
    grep -q "verify-issue-668" AGENTS.md
    grep -q "P668-1–P668-9" docs/frontend.md
    grep -q "Hourly / Daily / Monthly" docs/frontend.md
    test -f indexer/migrations/20260826180000_protocol_volume_hourly_monthly.sql
    grep -q "protocol_hourly_volume" indexer/migrations/20260826180000_protocol_volume_hourly_monthly.sql
    grep -q "protocol_monthly_volume" indexer/migrations/20260826180000_protocol_volume_hourly_monthly.sql
  '

run_step "source: grain allowlist; no Llama N+1; no GET swap_events; no PriceChart" \
  bash -c '
    set -euo pipefail
    grep -q parse_volume_grain indexer/src/db/queries/protocol_volume.rs
    grep -q refresh_protocol_hourly indexer/src/indexer/volume_aggregator.rs
    grep -q refresh_protocol_monthly indexer/src/indexer/volume_aggregator.rs
    grep -q protocol-volume-grain- frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx
    grep -q protocol-volume-chart-yaxis frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx
    grep -q protocol-volume-chart-tooltip frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx
    grep -q getProtocolVolumeSeries frontend-dapp/src/services/indexer/client.ts
    grep -q limitFromPlotWidth frontend-dapp/src/utils/protocolVolumeGrain.ts
    grep -q timeLabelIndexes frontend-dapp/src/utils/protocolVolumeGrain.ts
    grep -q timeLabelIndexes frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx
    if grep -nE "maxLabels = 5|sparseTimeLabelIndexes" \
         frontend-dapp/src/utils/protocolVolumeGrain.ts \
         frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx 2>/dev/null; then
      echo "Do not keep a global maxLabels = 5 / sparseTimeLabelIndexes" >&2
      exit 1
    fi
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
    if grep -nE "PriceChart|price-chart" \
         frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx \
         frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx 2>/dev/null; then
      echo "Do not mount PriceChart on the Protocol volume census chart" >&2
      exit 1
    fi
    if grep -nE "card-glass" frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx 2>/dev/null; then
      echo "Do not wrap the plot in card-glass" >&2
      exit 1
    fi
    if grep -nE "protocol-volume-daily-7d|protocol-volume-daily-30d" \
         frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx 2>/dev/null; then
      echo "Grain selector must not be 7d/30d" >&2
      exit 1
    fi
  '

if [[ "${VERIFY_ISSUE_668_SKIP_INDEXER:-}" == "1" ]]; then
  echo ""
  echo "[indexer lib + integration] skipped (VERIFY_ISSUE_668_SKIP_INDEXER=1)"
  ok "indexer lib + integration (skipped)"
else
  run_step "indexer lib: grain + days allowlist" \
    bash -c 'cd indexer && cargo test --lib protocol_volume -- --quiet'

  run_step "indexer integration: volume Δ% + daily alias + grain" \
    bash -c 'cd indexer && cargo test --test indexer_protocol_volume -- --test-threads=1 --quiet'
fi

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

run_step "frontend: Protocol grain chart + copy + client allowlist" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/ProtocolPage.test.tsx \
    src/utils/__tests__/protocolVolumeGrain.test.ts \
    src/utils/__tests__/trailingWindowCopy.test.ts \
    src/utils/__tests__/formatProtocolStats.test.ts \
    src/services/indexer/__tests__/client.test.ts'

if [ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]; then
  COMMON="$(git rev-parse --git-common-dir)"
  MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
  if [ -f "$MAIN_ROOT/frontend-dapp/.env.local" ] && [ "$MAIN_ROOT" != "$REPO_ROOT" ]; then
    cp "$MAIN_ROOT/frontend-dapp/.env.local" "$REPO_ROOT/frontend-dapp/.env.local"
    echo ""
    echo "[bootstrap] copied frontend-dapp/.env.local from primary checkout (gitignored)"
  fi
fi

if [[ "${VERIFY_ISSUE_668_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] skipped (VERIFY_ISSUE_668_SKIP_E2E=1)"
  ok "playwright e2e-smoke protocol-page (skipped)"
elif [ -d "$REPO_ROOT/frontend-dapp/node_modules/@playwright/test" ] || [ -d "$REPO_ROOT/frontend-dapp/node_modules/playwright" ]; then
  run_step "playwright e2e-smoke protocol-page (5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30668 PLAYWRIGHT_BASE_URL=http://127.0.0.1:30668 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/protocol-page.spec.ts'
else
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] SKIP (no Playwright install)"
  ok "playwright e2e-smoke protocol-page (skipped — no node_modules playwright)"
fi

if [[ "${VERIFY_ISSUE_668_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related 652/550/586] skipped (VERIFY_ISSUE_668_SKIP_RELATED=1)"
  ok "related verifies (skipped)"
else
  run_step "related: verify-issue-652" \
    bash -c 'VERIFY_ISSUE_652_SKIP_E2E=1 VERIFY_ISSUE_652_SKIP_RELATED=1 make verify-issue-652'
  run_step "related: verify-issue-550" \
    bash -c 'VERIFY_ISSUE_550_SKIP_E2E=1 VERIFY_ISSUE_550_SKIP_RELATED=1 make verify-issue-550'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
