#!/usr/bin/env bash
# Verification for GitLab #713 — Swap URL sync, reverse quotes, Share, Create/Trade prefill.
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

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #713 — Swap URL sync / reverse / Share / Create+Trade"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: canonical search + exactField + create/trade helpers + share" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/swapQueryParams.test.ts \
    src/utils/__tests__/createPairQuery.test.ts \
    src/utils/__tests__/tradeQueryResolve.test.ts \
    src/utils/__tests__/sharePageLink.test.ts \
    src/utils/__tests__/tradePairRoute.test.ts'

run_step "frontend: SwapPage rewrite/share + CreatePair prefill RTL" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/SwapPage.queryParams.test.tsx \
    src/pages/CreatePairPage.test.tsx'

run_step "frontend: Trade from/to resolve RTL" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/TradePage.test.tsx -t "713"'

run_step "code: canonical write-back, reverse sim, Share, create/trade parsers" \
  bash -c '
    set -euo pipefail
    grep -q "canonicalSwapSearch" frontend-dapp/src/pages/SwapPage.tsx
    grep -q "setSearchParams" frontend-dapp/src/pages/SwapPage.tsx
    grep -q "replace: true" frontend-dapp/src/pages/SwapPage.tsx
    grep -q "reverseSimulateSwap" frontend-dapp/src/pages/SwapPage.tsx
    grep -q "data-testid=\"swap-share-link\"" frontend-dapp/src/pages/SwapPage.tsx
    grep -q "buildCanonicalSwapShareUrl" frontend-dapp/src/utils/sharePageLink.ts
    grep -q "parseCreatePairQuery" frontend-dapp/src/pages/CreatePairPage.tsx
    grep -q "resolveTradePairFromQuery" frontend-dapp/src/pages/TradePage.tsx
    ! grep -q "window.location.href" frontend-dapp/src/pages/SwapPage.tsx
    ! grep -q "VITE_PUBLIC_ORIGIN" frontend-dapp/src/utils/sharePageLink.ts
    ! grep -q "dangerouslySetInnerHTML" frontend-dapp/src/pages/SwapPage.tsx frontend-dapp/src/utils/sharePageLink.ts
  '

run_step "guard: check_chrome_nesting.py" \
  python3 scripts/check_chrome_nesting.py

run_step "docs: Q713 + C542-11 prefill + TS-2 Swap exception" \
  bash -c '
    set -euo pipefail
    grep -q "swap-url-sync" docs/frontend.md
    grep -qE "\*\*Q713-1\*\*" docs/frontend.md
    grep -qE "\*\*Q713-10\*\*" docs/frontend.md
    grep -q "may prefill" docs/frontend.md
    grep -q "buildCanonicalSwapShareUrl" docs/frontend.md
    grep -q "verify-issue-713" docs/testing.md
  '

run_step "skill: AGENTS_FRONTEND_SWAP_URL_SYNC + crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_SWAP_URL_SYNC.md
    grep -qE "\*\*Q713-1" skills/AGENTS_FRONTEND_SWAP_URL_SYNC.md
    grep -qE "\*\*Q713-10" skills/AGENTS_FRONTEND_SWAP_URL_SYNC.md
    grep -q "make verify-issue-713" skills/AGENTS_FRONTEND_SWAP_URL_SYNC.md
    grep -q "AGENTS_FRONTEND_SWAP_URL_SYNC" skills/AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md
    grep -q "#713" skills/AGENTS_FRONTEND_SHARE_LINK.md
    grep -q "#713" skills/AGENTS_FRONTEND_CREATE_PAIR_PICKER.md
    grep -q "AGENTS_FRONTEND_SWAP_URL_SYNC" AGENTS.md
    grep -q "verify-issue-713" AGENTS.md
    grep -q "verify-issue-713" Makefile
  '

factory_on_chain() {
  local envf="$REPO_ROOT/frontend-dapp/.env.local"
  test -f "$envf" || return 1
  local addr lcd body
  addr="$(grep -E '^VITE_FACTORY_ADDRESS=' "$envf" | tail -1 | cut -d= -f2- | tr -d '"' | tr -d "'")"
  lcd="$(grep -E '^VITE_TERRA_LCD_URL=' "$envf" | tail -1 | cut -d= -f2- | tr -d '"' | tr -d "'" || true)"
  lcd="${lcd:-http://127.0.0.1:1317}"
  [[ -n "$addr" ]] || return 1
  # shellcheck source=scripts/lib/localterra-host-curl.sh
  source "$REPO_ROOT/scripts/lib/localterra-host-curl.sh"
  body="$(localterra_host_curl "${lcd%/}/cosmwasm/wasm/v1/contract/${addr}" 2>/dev/null || true)"
  printf '%s' "$body" | grep -qE '"contract_info"|"contract_address"'
}

if make -s has-localterra >/dev/null 2>&1 && test -f frontend-dapp/.env.local; then
  if factory_on_chain; then
    run_step "playwright: swap URL sync smoke (5 workers)" \
      bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-3016}" bash scripts/with-node.sh --cwd frontend-dapp -- \
        ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/swap-url-sync-713.spec.ts'
  else
    run_step "playwright: Create Pair query smoke (factory pin stale on this LocalTerra)" \
      bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-3016}" bash scripts/with-node.sh --cwd frontend-dapp -- \
        ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/swap-url-sync-713.spec.ts -g "create"'
    echo "  note: Swap URL/Share e2e needs a live VITE_FACTORY_ADDRESS (make deploy-local)"
  fi
else
  echo ""
  echo "[playwright: swap URL sync smoke (5 workers)]"
  echo "  SKIP (need LocalTerra + frontend-dapp/.env.local — unit + docs still required)"
  ok "playwright: skipped (no LocalTerra env)"
fi

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  PASS=$PASS  FAIL=$FAIL"
echo "────────────────────────────────────────────────────────────────"
if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
echo "All #713 checks passed."
