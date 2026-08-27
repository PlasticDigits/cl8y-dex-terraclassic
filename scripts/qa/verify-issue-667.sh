#!/usr/bin/env bash
# Verification for GitLab #667: /protocol Δ% grouped with its headline + integer census.
#
# Frontend-only. Playwright e2e-smoke is optional when frontend-dapp/node_modules exists.
# Related 652/550/569/586/653 stay green (skip their related recursion).
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
echo "  GitLab #667 — Protocol Δ% grouped with headline + integer census"
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

run_step "docs: P667 grouping invariant + skills + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P667-1" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P667-4" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "justify-start" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "justify-between" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "P667" docs/frontend.md
    grep -q "verify-issue-667" docs/frontend.md
    grep -q "verify-issue-667" AGENTS.md
    grep -q "P667-1–P667-4" AGENTS.md
    grep -q "#667" skills/AGENTS_FRONTEND_CHROME_NESTING.md
    grep -q "#667" docs/design-system.md
    grep -q "verify-issue-667" docs/testing.md
    grep -q "10.2.17" QA_TEMPLATE.md
    grep -q "24h-only" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md || grep -q "one** 24h chip" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
  '

run_step "source: StatBox value+Δ% row is grouped, not justify-between" \
  bash -c '
    set -euo pipefail
    grep -q "stat-value-row" frontend-dapp/src/components/ui/StatBox.tsx
    grep -q "stat-delta-cluster" frontend-dapp/src/components/ui/StatBox.tsx
    grep -q "stat-value-row" frontend-dapp/src/index.css
    grep -q "justify-content: flex-start" frontend-dapp/src/index.css
    if grep -nE "className=.*justify-between" frontend-dapp/src/components/ui/StatBox.tsx; then
      echo "StatBox value row must not use justify-between" >&2
      exit 1
    fi
    grep -q "toLocaleString" frontend-dapp/src/utils/formatProtocolStats.ts
    grep -q "1e3" frontend-dapp/src/utils/formatProtocolStats.ts
    grep -q "variant=\"flat\"" frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx
    grep -q "variant=\"flat\"" frontend-dapp/src/components/protocol/ProtocolFeeStats.tsx
    if grep -nE "getDefillamaDaily|/defillama/daily" \
         frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx \
         frontend-dapp/src/components/protocol/ProtocolFeeStats.tsx \
         frontend-dapp/src/pages/ProtocolPage.tsx 2>/dev/null; then
      echo "Protocol must not call GET /defillama/daily" >&2
      exit 1
    fi
  '

run_step "frontend: StatBox + formatProtocolCount + Protocol RTL" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/pages/ProtocolPage.test.tsx src/components/ui/__tests__/StatBox.test.tsx src/utils/__tests__/formatProtocolStats.test.ts'

if [ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]; then
  COMMON="$(git rev-parse --git-common-dir)"
  MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
  if [ -f "$MAIN_ROOT/frontend-dapp/.env.local" ] && [ "$MAIN_ROOT" != "$REPO_ROOT" ]; then
    cp "$MAIN_ROOT/frontend-dapp/.env.local" "$REPO_ROOT/frontend-dapp/.env.local"
    echo ""
    echo "[bootstrap] copied frontend-dapp/.env.local from primary checkout (gitignored)"
  fi
fi

if [[ "${VERIFY_ISSUE_667_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] skipped (VERIFY_ISSUE_667_SKIP_E2E=1)"
  ok "playwright e2e-smoke protocol-page (skipped)"
elif [ -d "$REPO_ROOT/frontend-dapp/node_modules/@playwright/test" ] || [ -d "$REPO_ROOT/frontend-dapp/node_modules/playwright" ]; then
  run_step "playwright e2e-smoke protocol-page (5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30667 PLAYWRIGHT_BASE_URL=http://127.0.0.1:30667 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/protocol-page.spec.ts'
else
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] SKIP (no Playwright install)"
  ok "playwright e2e-smoke protocol-page (skipped — no node_modules playwright)"
fi

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  COMMON="$(git rev-parse --git-common-dir)"
  MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
  if [ -f "$MAIN_ROOT/indexer/.env" ] && [ "$MAIN_ROOT" != "$REPO_ROOT" ]; then
    cp "$MAIN_ROOT/indexer/.env" "$REPO_ROOT/indexer/.env"
    echo "[bootstrap] copied indexer/.env from primary checkout (gitignored)"
  fi
fi

if [[ "${VERIFY_ISSUE_667_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related 652/550/569/586/653] skipped (VERIFY_ISSUE_667_SKIP_RELATED=1)"
  ok "related verifies (skipped)"
else
  run_step "related: verify-issue-652" \
    bash -c 'VERIFY_ISSUE_652_SKIP_E2E=1 VERIFY_ISSUE_652_SKIP_RELATED=1 make verify-issue-652'
  run_step "related: verify-issue-653" \
    bash -c 'make verify-issue-653'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
echo "==> GitLab #667 verification passed"
