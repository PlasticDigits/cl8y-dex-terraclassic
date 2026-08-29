#!/usr/bin/env bash
# Verification for GitLab #703: /protocol Monthly UTC chart — phone x-axis
# overlap (last 12 months + YY-MM, no rotated ticks).
#
# Frontend-only. Playwright e2e-smoke is optional when frontend-dapp/node_modules exists.
# Related 677/668/689 stay green (skip their related recursion).
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
echo "  GitLab #703 — Protocol Monthly phone x-axis (YY-MM, ≤12 months)"
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

run_step "docs: P703 / amended P668-4 P668-9 + skills + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P703-1" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P703-8" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P703-2" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "YY-MM" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "do not rotate" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "phone-width plots requests" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P703-1–P703-8" docs/frontend.md
    grep -q "verify-issue-703" docs/frontend.md
    grep -q "verify-issue-703" AGENTS.md
    grep -q "P703-1–P703-8" AGENTS.md
    grep -q "P703-1–P703-4" skills/AGENTS_FRONTEND_TRAILING_WINDOW.md
    grep -q "verify-issue-703" docs/testing.md
    grep -q "10.2.20" QA_TEMPLATE.md
    grep -q "#703" QA_TEMPLATE.md
    grep -q "make verify-issue-703" docs/indexer-invariants.md
  '

run_step "source: monthly phone cap; YY-MM axis; no rotate; no Llama/PriceChart/card-glass" \
  bash -c '
    set -euo pipefail
    grep -q PROTOCOL_VOLUME_MONTHLY_NARROW_MAX frontend-dapp/src/utils/protocolVolumeGrain.ts
    grep -q monthlyLimitForPlotWidth frontend-dapp/src/utils/protocolVolumeGrain.ts
    grep -q "YY-MM" frontend-dapp/src/utils/protocolVolumeGrain.ts
    grep -q formatPeriodAxisLabel frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx
    grep -q "transform: '\''none'\''" frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx
    grep -q assertMonthlyXaxisNoOverlap frontend-dapp/e2e/protocol-page.spec.ts
    grep -q "GitLab #703" frontend-dapp/e2e/protocol-page.spec.ts
    if grep -nE "transform=\"rotate|textPath|writing-mode|writingMode" \
         frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx 2>/dev/null; then
      echo "Do not rotate or writing-mode x-axis tick text" >&2
      exit 1
    fi
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
    if grep -nE "PriceChart|price-chart" \
         frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx \
         frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx 2>/dev/null; then
      echo "Do not mount PriceChart on the Protocol census chart" >&2
      exit 1
    fi
    if grep -nE "card-glass" frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx 2>/dev/null; then
      echo "Do not wrap the plot in card-glass" >&2
      exit 1
    fi
    if grep -nE "innerHTML" frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx 2>/dev/null; then
      echo "Tooltip must not use innerHTML" >&2
      exit 1
    fi
    if grep -nE "protocol-volume-daily-7d|protocol-volume-daily-30d" \
         frontend-dapp/src/components/protocol/ProtocolVolumeDailyChart.tsx 2>/dev/null; then
      echo "Grain selector must not be 7d/30d" >&2
      exit 1
    fi
  '

run_step "frontend: Protocol Monthly YY-MM + grain helper + allowlist" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/ProtocolPage.test.tsx \
    src/utils/__tests__/protocolVolumeGrain.test.ts \
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

if [[ "${VERIFY_ISSUE_703_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] skipped (VERIFY_ISSUE_703_SKIP_E2E=1)"
  ok "playwright e2e-smoke protocol-page (skipped)"
elif [ -d "$REPO_ROOT/frontend-dapp/node_modules/@playwright/test" ] || [ -d "$REPO_ROOT/frontend-dapp/node_modules/playwright" ]; then
  free_tcp_port 30703
  run_step "playwright e2e-smoke protocol-page (5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30703 PLAYWRIGHT_BASE_URL=http://127.0.0.1:30703 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/protocol-page.spec.ts'
else
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] SKIP (no Playwright install)"
  ok "playwright e2e-smoke protocol-page (skipped — no node_modules playwright)"
fi

if [[ "${VERIFY_ISSUE_703_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related 677/668/689] skipped (VERIFY_ISSUE_703_SKIP_RELATED=1)"
  ok "related verifies (skipped)"
else
  run_step "related: verify-issue-677" \
    bash -c 'VERIFY_ISSUE_677_SKIP_E2E=1 VERIFY_ISSUE_677_SKIP_RELATED=1 make verify-issue-677'
  run_step "related: verify-issue-668" \
    bash -c 'VERIFY_ISSUE_668_SKIP_E2E=1 VERIFY_ISSUE_668_SKIP_RELATED=1 VERIFY_ISSUE_668_SKIP_INDEXER=1 make verify-issue-668'
  run_step "related: verify-issue-689" \
    bash -c 'VERIFY_ISSUE_689_SKIP_E2E=1 VERIFY_ISSUE_689_SKIP_RELATED=1 VERIFY_ISSUE_689_SKIP_INDEXER=1 make verify-issue-689'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
echo "==> GitLab #703 verification passed"
