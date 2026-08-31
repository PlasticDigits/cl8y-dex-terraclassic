#!/usr/bin/env bash
# Verification for GitLab #711 — Swap query params on / and /swap.
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
echo "  GitLab #711 — Swap query params"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: parser + alias redirect + SwapPage RTL + ust1 path" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/swapQueryParams.test.ts \
    src/utils/__tests__/tokenRegistry.test.ts \
    src/utils/__tests__/ust1SecondaryMarket.test.ts \
    src/components/common/__tests__/SwapAliasRedirect.test.tsx \
    src/pages/SwapPage.queryParams.test.tsx'

run_step "frontend: Create Pair C542-11 listed prefill + hostile ignore" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/CreatePairPage.test.tsx -t "C542-11"'

run_step "code: /swap alias preserves search; SwapPage apply-once" \
  bash -c '
    set -euo pipefail
    grep -q "SwapAliasRedirect" frontend-dapp/src/App.tsx
    grep -q "path=\"/swap\"" frontend-dapp/src/App.tsx
    grep -q "applySwapQueryParams" frontend-dapp/src/pages/SwapPage.tsx
    grep -q "useSearchParams" frontend-dapp/src/pages/SwapPage.tsx
    grep -q "appliedSwapQueryKeyRef" frontend-dapp/src/pages/SwapPage.tsx
    grep -q "from=" frontend-dapp/src/utils/ust1SecondaryMarket.ts
    ! grep -q "does not yet honor token query params" frontend-dapp/src/utils/ust1SecondaryMarket.ts
  '

run_step "docs: Q711 invariants + supported keys" \
  bash -c '
    set -euo pipefail
    grep -q "swap-query-params" docs/frontend.md
    grep -qE "\*\*Q711-1\*\*" docs/frontend.md
    grep -qE "\*\*Q711-8\*\*" docs/frontend.md
    grep -q "inputCurrency" docs/frontend.md
    grep -q "outputCurrency" docs/frontend.md
    grep -q "verify-issue-711" docs/testing.md
  '

run_step "skill: AGENTS_FRONTEND_SWAP_QUERY_PARAMS + crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md
    grep -qE "\*\*Q711-1" skills/AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md
    grep -qE "\*\*Q711-8" skills/AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md
    grep -q "make verify-issue-711" skills/AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md
    grep -q "#711" skills/AGENTS_FRONTEND_SHELL_NAV.md
    grep -q "AGENTS_FRONTEND_SWAP_QUERY_PARAMS" skills/AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md
    grep -q "AGENTS_FRONTEND_SWAP_QUERY_PARAMS" skills/AGENTS_UST1_SECONDARY_AMM.md
    grep -q "#711" skills/AGENTS_FRONTEND_CREATE_PAIR_PICKER.md
    grep -q "AGENTS_FRONTEND_SWAP_QUERY_PARAMS" AGENTS.md
    grep -q "verify-issue-711" AGENTS.md
    grep -q "verify-issue-711" Makefile
  '

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  PASS=$PASS  FAIL=$FAIL"
echo "────────────────────────────────────────────────────────────────"
if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
echo "All #711 checks passed."
