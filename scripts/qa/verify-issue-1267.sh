#!/usr/bin/env bash
# Automated verification for Forgejo #1267 — extra-debit Sell TaxPreview.debit > balance.
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
echo "  Forgejo #1267 — extra-debit Sell submit gate"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: extra-debit gate / Max / hook / humanize / Swap / Trade RTL" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/taxPreviewMaxSpend.test.ts \
    src/utils/__tests__/maxSpendableAmount.test.ts \
    src/utils/__tests__/humanizeTerraTxError.test.ts \
    src/hooks/__tests__/useCommunityTaxSellBps.test.tsx \
    src/pages/SwapPage.extraDebitSell.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.extraDebitSell.test.tsx'

run_step "code: Swap + Trade call extraDebitSubmitGate" \
  bash -c 'grep -q extraDebitSubmitGate frontend-dapp/src/pages/SwapPage.tsx && \
    grep -q extraDebitSubmitGate frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx && \
    grep -q extraDebitSubmitGate frontend-dapp/src/utils/taxPreviewMaxSpend.ts'

run_step "code: sell detection is GetConfig sell_bps not pin equality" \
  bash -c 'grep -q parseCommunityTaxSellBps frontend-dapp/src/hooks/useCommunityTaxSellBps.ts && \
    ! grep -qE "COMMUNITY_TAX_CODE_ID.*===|code_id === COMMUNITY_TAX" frontend-dapp/src/hooks/useCommunityTaxSellBps.ts && \
    grep -q "Catalog pin equality is \*\*not\*\* the sell detector" frontend-dapp/src/hooks/useCommunityTaxSellBps.ts'

run_step "code: no pair/router FoT / wasm change in this ticket" \
  bash -c 'grep -q "S1267-8" skills/AGENTS_FRONTEND_EXTRA_DEBIT_SELL.md && \
    grep -q "Do \\*\\*not\\*\\* add FoT math" skills/AGENTS_FRONTEND_EXTRA_DEBIT_SELL.md && \
    grep -q "Do not change pair or router contracts" skills/AGENTS_FRONTEND_EXTRA_DEBIT_SELL.md'

run_step "docs: S1267 + testing + skill + AGENTS" \
  bash -c 'grep -qE "S1267-1" docs/frontend.md && \
    grep -q "verify-issue-1267" docs/testing.md && \
    grep -q "S1267-1" skills/AGENTS_FRONTEND_EXTRA_DEBIT_SELL.md && \
    grep -q "AGENTS_FRONTEND_EXTRA_DEBIT_SELL" AGENTS.md && \
    grep -q "S1267-1" docs/contracts-security-audit.md'

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> Forgejo #1267 verification passed"
