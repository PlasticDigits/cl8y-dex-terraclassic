#!/usr/bin/env bash
# Verification for GitLab #677: /protocol leftovers — liquidity 24h-only Δ%
# + denser UTC volume x-axis labels.
#
# Frontend-only. Playwright e2e-smoke is optional when frontend-dapp/node_modules exists.
# Related 667/668/652 stay green (skip their related recursion).
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
echo "  GitLab #677 — Protocol leftovers: 24h liquidity + dense x-axis"
echo "════════════════════════════════════════════════════════════════"

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

run_step "docs: P569-1 / P667-2 / P668-9 + skills + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P569-1" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P667-2" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P668-9" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "24h-only" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "protocol-stat-liquidity-24h" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "step 1" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P668-9" docs/frontend.md
    grep -q "verify-issue-677" docs/frontend.md
    grep -q "verify-issue-677" AGENTS.md
    grep -q "P668-1–P668-9" AGENTS.md
    grep -q "#677" skills/AGENTS_FRONTEND_CHROME_NESTING.md
    grep -q "#677" docs/design-system.md
    grep -q "P668-9" skills/AGENTS_FRONTEND_TRAILING_WINDOW.md
    grep -q "verify-issue-677" docs/testing.md
    grep -q "10.2.18" QA_TEMPLATE.md
    grep -q "#677" QA_TEMPLATE.md
  '

run_step "source: 24h-only liquidity tile; grain-aware x-axis; no maxLabels=5" \
  bash -c '
    set -euo pipefail
    grep -q "protocol-stat-liquidity-24h" frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx
    if grep -nE "protocol-stat-liquidity-30d" frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx; then
      echo "Total liquidity must not render protocol-stat-liquidity-30d" >&2
      exit 1
    fi
    grep -q "24h liquidity is vs indexer snapshots" frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx
    grep -q "timeLabelIndexes" frontend-dapp/src/utils/protocolVolumeGrain.ts
    grep -q "timeLabelStep" frontend-dapp/src/utils/protocolVolumeGrain.ts
    grep -q "timeLabelIndexes" frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx
    if grep -nE "maxLabels = 5|sparseTimeLabelIndexes" \
         frontend-dapp/src/utils/protocolVolumeGrain.ts \
         frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx 2>/dev/null; then
      echo "Do not keep a global maxLabels = 5 / sparseTimeLabelIndexes" >&2
      exit 1
    fi
    grep -q "variant=\"flat\"" frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx
    if grep -nE "getDefillamaDaily|/defillama/daily" \
         frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx \
         frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx \
         frontend-dapp/src/pages/ProtocolPage.tsx 2>/dev/null; then
      echo "Protocol must not call GET /defillama/daily" >&2
      exit 1
    fi
    if grep -nE "PriceChart|price-chart" \
         frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx \
         frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx 2>/dev/null; then
      echo "Do not mount PriceChart on the Protocol volume census chart" >&2
      exit 1
    fi
  '

run_step "frontend: Protocol RTL + grain helper + StatBox" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/ProtocolPage.test.tsx \
    src/utils/__tests__/protocolVolumeGrain.test.ts \
    src/components/ui/__tests__/StatBox.test.tsx'

if [ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]; then
  COMMON="$(git rev-parse --git-common-dir)"
  MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
  if [ -f "$MAIN_ROOT/frontend-dapp/.env.local" ] && [ "$MAIN_ROOT" != "$REPO_ROOT" ]; then
    cp "$MAIN_ROOT/frontend-dapp/.env.local" "$REPO_ROOT/frontend-dapp/.env.local"
    echo ""
    echo "[bootstrap] copied frontend-dapp/.env.local from primary checkout (gitignored)"
  fi
fi

# Dedicated Playwright Vite on :30677. A leftover process from a previous
# leftover/child run makes Playwright fail closed (`reuseExistingServer` is
# false when PLAYWRIGHT_WEB_PORT is set).
free_tcp_port() {
  local port="$1"
  local pids=""
  if command -v lsof >/dev/null 2>&1; then
    pids="$(lsof -ti ":$port" 2>/dev/null || true)"
  elif command -v fuser >/dev/null 2>&1; then
    pids="$(fuser "$port/tcp" 2>/dev/null || true)"
  fi
  if [[ -n "${pids// /}" ]]; then
    echo "[bootstrap] freeing TCP :$port (stale Playwright Vite): $pids"
    # shellcheck disable=SC2086
    kill $pids 2>/dev/null || true
    sleep 1
  fi
}

if [[ "${VERIFY_ISSUE_677_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] skipped (VERIFY_ISSUE_677_SKIP_E2E=1)"
  ok "playwright e2e-smoke protocol-page (skipped)"
elif [ -d "$REPO_ROOT/frontend-dapp/node_modules/@playwright/test" ] || [ -d "$REPO_ROOT/frontend-dapp/node_modules/playwright" ]; then
  free_tcp_port 30677
  run_step "playwright e2e-smoke protocol-page (5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30677 PLAYWRIGHT_BASE_URL=http://127.0.0.1:30677 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/protocol-page.spec.ts'
else
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] SKIP (no Playwright install)"
  ok "playwright e2e-smoke protocol-page (skipped — no node_modules playwright)"
fi

if [[ "${VERIFY_ISSUE_677_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related 667/668/652] skipped (VERIFY_ISSUE_677_SKIP_RELATED=1)"
  ok "related verifies (skipped)"
else
  run_step "related: verify-issue-667" \
    bash -c 'VERIFY_ISSUE_667_SKIP_E2E=1 VERIFY_ISSUE_667_SKIP_RELATED=1 make verify-issue-667'
  run_step "related: verify-issue-668" \
    bash -c 'VERIFY_ISSUE_668_SKIP_E2E=1 VERIFY_ISSUE_668_SKIP_RELATED=1 VERIFY_ISSUE_668_SKIP_INDEXER=1 make verify-issue-668'
  run_step "related: verify-issue-652" \
    bash -c 'VERIFY_ISSUE_652_SKIP_E2E=1 VERIFY_ISSUE_652_SKIP_RELATED=1 VERIFY_ISSUE_652_SKIP_INDEXER=1 make verify-issue-652'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
echo "==> GitLab #677 verification passed"
