#!/usr/bin/env bash
# Verification for GitLab #678 — Swap/Trade unfunded pay + UST1 acquire guidance.
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
echo "  GitLab #678 — Swap acquire guidance"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: inverse math + helper + prefill + Swap/Trade/Ust1" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/ust1WindowMath.test.ts \
    src/utils/__tests__/ust1AcquirePrefill.test.ts \
    src/utils/swapPayAcquireGuidance.test.ts \
    src/pages/SwapPage.acquireGuidance.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.acquireGuidance.test.tsx \
    src/pages/Ust1Page.test.tsx'

run_step "code: shared helper + banner + hook exist" \
  bash -c 'test -f frontend-dapp/src/utils/swapPayAcquireGuidance.ts && \
    test -f frontend-dapp/src/components/swap/SwapPayAcquireGuidanceBanner.tsx && \
    test -f frontend-dapp/src/hooks/useSwapPayAcquireGuidance.ts && \
    test -f frontend-dapp/src/utils/ust1AcquirePrefill.ts'

run_step "code: inverse deposit + Swap/Trade consume helper" \
  bash -c 'grep -qE "vfdusdInForTargetUst1" frontend-dapp/src/utils/ust1WindowMath.ts && \
    grep -qE "useSwapPayAcquireGuidance" frontend-dapp/src/pages/SwapPage.tsx && \
    grep -qE "useSwapPayAcquireGuidance" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx && \
    grep -qE "parseUst1AcquirePrefill" frontend-dapp/src/pages/Ust1Page.tsx'

run_step "code: quote-only + Guide testids" \
  bash -c 'grep -qE "swap-quote-only" frontend-dapp/src/pages/SwapPage.tsx && \
    grep -qE "trade-market-quote-only" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx && \
    grep -qE "acquire-guide" frontend-dapp/src/components/swap/SwapPayAcquireGuidanceBanner.tsx'

run_step "docs: swap-max-spread + ust1-window acquire notes" \
  bash -c 'grep -qE "A678-1|#678" docs/swap-max-spread-ux.md && \
    grep -qE "U9|#678" docs/runbooks/ust1-window-ui.md && \
    grep -qE "A678-1" docs/frontend.md'

run_step "skill: AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE + AGENTS.md" \
  bash -c 'grep -qE "A678-1" skills/AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md && \
    grep -qE "AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE" AGENTS.md && \
    grep -qE "verify-issue-678" AGENTS.md'

run_step "skill: window + U1 + copy crosslinks" \
  bash -c 'grep -qE "#678" skills/AGENTS_UST1_WINDOW_UI.md && \
    grep -qE "#678" skills/AGENTS_UST1_SECONDARY_AMM.md && \
    grep -qE "#678" skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md'

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  PASS=$PASS  FAIL=$FAIL"
echo "────────────────────────────────────────────────────────────────"
if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
echo "All #678 checks passed."
