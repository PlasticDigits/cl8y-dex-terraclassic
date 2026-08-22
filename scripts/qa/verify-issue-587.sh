#!/usr/bin/env bash
# Verification for GitLab #587 — wrap+≥2hop LUNC↔USTR gas + Swap Network fee.
#
# Layers (no LocalTerra required for unit/docs):
#   1. Frontend unit: envelope, Max, inventory, Swap RTL, hint, humanize, gate
#   2. Constant / combo-overhead floors vs documented 2.31M gem sum
#   3. Docs/skills #587 crosslinks
#   4. Retest unit + docs
#
# Optional chain: Playwright wrap-swap E7/E8 (make setup-cloud-localterra first).
#
# Refs: skills/AGENTS_TERRACLASSIC_GAS.md, docs/frontend.md § Terra Classic gas limits
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
echo "  GitLab #587 — wrap+≥2hop gas + Swap Network fee (LUNC)"
echo "════════════════════════════════════════════════════════════════"

run_frontend_unit() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/services/terraclassic/__tests__/transactions.test.ts \
    src/services/terraclassic/__tests__/terraGas.retailShapes.test.ts \
    src/services/terraclassic/__tests__/swapNetworkFee.test.ts \
    src/services/terraclassic/__tests__/terraClassicFeeEstimate.test.ts \
    src/utils/__tests__/maxSpendableAmount.test.ts \
    src/utils/__tests__/swapNativeGasBalanceGate.test.ts \
    src/utils/__tests__/humanizeTerraTxError.test.ts \
    src/components/common/__tests__/TerraClassicTxFeeHint.test.tsx \
    src/pages/SwapPage.test.tsx
}

run_docs() {
  set -euo pipefail
  rg -q 'WRAP_ROUTER_COMBO_OVERHEAD_GAS' frontend-dapp/src/utils/constants.ts
  rg -q 'SendHookGasDecodeError' frontend-dapp/src/services/terraclassic/terraGas.ts
  rg -q 'RETAIL_COMBINED_ENVELOPE_FIXTURES' frontend-dapp/src/services/terraclassic/terraGasRetailInventory.ts
  rg -q 'swap-network-fee' frontend-dapp/src/pages/SwapPage.tsx
  rg -q '#587' docs/frontend.md
  rg -q '950k' docs/frontend.md
  rg -q 'WRAP_ROUTER_COMBO_OVERHEAD_GAS' docs/frontend.md
  rg -q '#587' skills/AGENTS_TERRACLASSIC_GAS.md
  rg -q 'make verify-issue-587' skills/AGENTS_TERRACLASSIC_GAS.md
  rg -q 'AGENTS_TERRACLASSIC_GAS' AGENTS.md
  rg -q 'verify-issue-587' AGENTS.md
  rg -q 'verify-issue-587' docs/testing.md
  rg -q '#587' NATIVE_TOKEN_WRAPPING.md
  rg -q 'paid in \*\*LUNC\*\*' docs/user-incident-faq.md
  rg -q 'estimateSwapNetworkFee' frontend-dapp/src/services/terraclassic/swapNetworkFee.ts
  rg -q 'evaluateSwapNativeGasGate' frontend-dapp/src/utils/swapNativeGasBalanceGate.ts
}

echo ""
echo "── first pass ──"
run_step "frontend unit: wrap+2hop envelope + Swap Network fee" \
  run_frontend_unit

run_step "docs: #587 + skill + crosslinks" \
  run_docs

echo ""
echo "── retest ──"
run_step "retest frontend unit" \
  run_frontend_unit

run_step "retest docs #587" \
  run_docs

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #587 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
