#!/usr/bin/env bash
# Automated verification for Forgejo #1285 — TaxPreview send_msg + hop_trader_debit leftover (#1267).
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
echo "  Forgejo #1285 — TaxPreview execute-aligned preview"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: preview query + hop debit + hook tests" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/communityTaxPreviewQuery.test.ts \
    src/utils/taxPreviewMaxSpend.test.ts \
    src/hooks/__tests__/useCommunityTaxSellBps.test.tsx \
    src/pages/SwapPage.extraDebitSell.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.extraDebitSell.test.tsx'

run_step "code: resolveCommunityTaxPreviewQuery wired on Swap + Trade" \
  bash -c 'grep -q resolveCommunityTaxPreviewQuery frontend-dapp/src/pages/SwapPage.tsx && \
    grep -q resolveCommunityTaxPreviewQuery frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx && \
    grep -q taxPreviewExecuteDebitRaw frontend-dapp/src/hooks/useCommunityTaxSellBps.ts'

run_step "code: inline Swap/Trade tx errors humanized" \
  bash -c 'grep -q "humanizeUserFacingErrorFromUnknown(swapMutation.error)" frontend-dapp/src/pages/SwapPage.tsx && \
    grep -q "humanizeUserFacingErrorFromUnknown(swapMutation.error)" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx'

run_step "docs: S1285 + skill + verify target" \
  bash -c 'grep -q "S1285-1" docs/frontend.md && \
    grep -q "verify-issue-1285" docs/testing.md && \
    grep -q "S1285-1" skills/AGENTS_FRONTEND_EXTRA_DEBIT_SELL.md && \
    grep -q "verify-issue-1285" AGENTS.md'

run_step "regression: #1267 verify still passes" \
  make verify-issue-1267

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> Forgejo #1285 verification passed"
