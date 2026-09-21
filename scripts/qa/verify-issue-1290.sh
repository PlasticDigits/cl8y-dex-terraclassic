#!/usr/bin/env bash
# Forgejo #1306 — leftover hub wrap cLUNC vs CEX LUNC (Q22 / B1290; related #1302 / !1290 / #1240).
# Extends verify-issue-1240 with hubPriceTicker Vitest + Playwright protocol-page (5 workers).
#
# Refs: docs/adr/0009-verify-issue-1290-hub-wrap-labels.md,
#       skills/AGENTS_FRONTEND_PROTOCOL_STATS.md (P1240),
#       frontend-dapp/e2e/protocol-page.spec.ts (P1/P2/P4 hub vs oracle casing)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

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

echo "════════════════════════════════════════════════════════════════"
echo "  #1306 — hub wrap cLUNC vs CEX LUNC (Q22 / B1290; child #1240)"
echo "════════════════════════════════════════════════════════════════"

run_step "verify-issue-1240 (P1240-1–P1240-8)" \
  ./scripts/qa/verify-issue-1240.sh

run_step "frontend: hubPriceTicker wrap labels + parse guard" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/hubPriceTicker.test.ts

if [ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]; then
  COMMON="$(git rev-parse --git-common-dir)"
  MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
  if [ -f "$MAIN_ROOT/frontend-dapp/.env.local" ] && [ "$MAIN_ROOT" != "$REPO_ROOT" ]; then
    cp "$MAIN_ROOT/frontend-dapp/.env.local" "$REPO_ROOT/frontend-dapp/.env.local"
    echo ""
    echo "[bootstrap] copied frontend-dapp/.env.local from primary checkout (gitignored)"
  fi
fi

if [[ "${VERIFY_ISSUE_1290_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] skipped (VERIFY_ISSUE_1290_SKIP_E2E=1)"
  ok "playwright e2e-smoke protocol-page (skipped)"
elif [ -d "$REPO_ROOT/frontend-dapp/node_modules/@playwright/test" ] || [ -d "$REPO_ROOT/frontend-dapp/node_modules/playwright" ]; then
  free_tcp_port 30129
  run_step "playwright e2e-smoke protocol-page (5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30129 PLAYWRIGHT_BASE_URL=http://127.0.0.1:30129 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/protocol-page.spec.ts'
else
  echo ""
  echo "[playwright e2e-smoke protocol-page (5 workers)] SKIP (no Playwright install)"
  ok "playwright e2e-smoke protocol-page (skipped — no node_modules playwright)"
fi

run_step "docs: #1306 verify crosslink (B1290-4)" \
  bash -c 'grep -qE "verify-issue-1290" docs/testing.md && \
  grep -qE "verify-issue-1290" AGENTS.md && \
  grep -qE "verify-issue-1290" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md && \
  grep -qE "verify-issue-1290" skills/AGENTS_FRONTEND_PROTOCOL_HUB.md && \
  grep -qE "verify-issue-1290" docs/frontend.md && \
  grep -qE "verify-issue-1290" docs/adr/0009-verify-issue-1290-hub-wrap-labels.md && \
  grep -qE "dex-hub-wrap-labels|verify-issue-1290" docs/architecture.md && \
  grep -qE "Q22|ops-hub-wrap-1302" docs/qa-invariants.md'

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  $PASS passed, $FAIL failed"
echo "────────────────────────────────────────────────────────────────"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
echo ""
echo "  #1306 hub wrap vs CEX native labels verified (aliases verify-issue-1302 / verify-issue-1306)."
