#!/usr/bin/env bash
# Verification for GitLab #715 — tokenlist unique symbols, Swap from=/to= symbols, Share logos.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0

ok()  { PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

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
echo "  GitLab #715 — Swap tokenlist symbols / unique ticker CI / Share logos"
echo "════════════════════════════════════════════════════════════════"

run_step "tokenlist: uniqueness (published + fail-closed fixtures)" \
  python3 scripts/qa/tokenlist_unique_symbols.py --self-test

run_step "frontend: tokenlist maps + swap query + share + ust1 path" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/tokenlistQueryCatalog.test.ts \
    src/utils/__tests__/swapQueryParams.test.ts \
    src/utils/__tests__/sharePageLink.test.ts \
    src/utils/__tests__/ust1SecondaryMarket.test.ts \
    src/utils/__tests__/tradeQueryResolve.test.ts \
    src/utils/__tests__/tokenRegistry.test.ts'

run_step "frontend: SwapPage symbols/share RTL + ShareLinkButton slot" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/SwapPage.queryParams.test.tsx \
    src/components/ui/__tests__/ShareLinkButton.test.tsx'

run_step "code: bundled lookup, symbol encode, Share logos, no location.href" \
  bash -c '
    set -euo pipefail
    grep -q "executeIdToQueryToken" frontend-dapp/src/utils/swapQueryParams.ts
    grep -q "queryTokenToExecuteId" frontend-dapp/src/utils/swapQueryParams.ts
    grep -q "tokenlistQueryCatalog" frontend-dapp/src/utils/createPairTokenCatalog.ts
    grep -q "SwapSharePairLabel" frontend-dapp/src/pages/SwapPage.tsx
    grep -q "buttonContent" frontend-dapp/src/components/ui/ShareLinkButton.tsx
    grep -q "swapDeepLinkPath" frontend-dapp/src/utils/ust1SecondaryMarket.ts
    ! grep -q "window.location.href" frontend-dapp/src/pages/SwapPage.tsx
    ! grep -q "VITE_PUBLIC_ORIGIN" frontend-dapp/src/utils/sharePageLink.ts
    ! grep -q "fetch(tokenlist" frontend-dapp/src/utils/tokenlistQueryCatalog.ts frontend-dapp/src/utils/createPairTokenCatalog.ts
  '

run_step "guard: check_chrome_nesting.py" \
  python3 scripts/check_chrome_nesting.py

run_step "docs: TL/QS/SH + tokenlist README + testing.md" \
  bash -c '
    set -euo pipefail
    grep -q "swap-tokenlist-symbols" docs/frontend.md
    grep -qE "\*\*TL-1\*\*" docs/frontend.md
    grep -qE "\*\*QS-6\*\*" docs/frontend.md
    grep -qE "\*\*SH-3\*\*" docs/frontend.md
    grep -q "unique" tokenlist/README.md
    grep -q "from=UST1" tokenlist/README.md
    grep -q "verify-issue-715" docs/testing.md
  '

run_step "skill: AGENTS_FRONTEND_SWAP_TOKENLIST_SYMBOLS + crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_SWAP_TOKENLIST_SYMBOLS.md
    grep -qE "\*\*TL-1" skills/AGENTS_FRONTEND_SWAP_TOKENLIST_SYMBOLS.md
    grep -qE "\*\*SH-3" skills/AGENTS_FRONTEND_SWAP_TOKENLIST_SYMBOLS.md
    grep -q "make verify-issue-715" skills/AGENTS_FRONTEND_SWAP_TOKENLIST_SYMBOLS.md
    grep -q "AGENTS_FRONTEND_SWAP_TOKENLIST_SYMBOLS" skills/AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md
    grep -q "AGENTS_FRONTEND_SWAP_TOKENLIST_SYMBOLS" skills/AGENTS_FRONTEND_SWAP_URL_SYNC.md
    grep -q "#715" skills/AGENTS_FRONTEND_SHARE_LINK.md
    grep -q "#715" skills/AGENTS_FRONTEND_CREATE_PAIR_PICKER.md
    grep -q "AGENTS_FRONTEND_SWAP_TOKENLIST_SYMBOLS" AGENTS.md
    grep -q "verify-issue-715" AGENTS.md
    grep -q "verify-issue-715" Makefile
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
    run_step "playwright: swap tokenlist symbols smoke (5 workers)" \
      bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-3016}" bash scripts/with-node.sh --cwd frontend-dapp -- \
        ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/swap-tokenlist-symbols-715.spec.ts'
  else
    echo ""
    echo "[playwright: swap tokenlist symbols smoke (5 workers)]"
    echo "  SKIP (factory pin stale — unit + docs still required)"
    ok "playwright: skipped (stale factory)"
  fi
else
  echo ""
  echo "[playwright: swap tokenlist symbols smoke (5 workers)]"
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
echo "All #715 checks passed."
