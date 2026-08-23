#!/usr/bin/env bash
# Verification for GitLab #599 — unwrap+≥2hop USTR→USTC gas combo.
#
# Layers (no LocalTerra required for unit/docs):
#   1. Frontend unit: envelope, Max, inventory, Swap RTL, hint, humanize, gate
#   2. Constant / unwrap-combo floors vs documented 2.71M OOG sum
#   3. Docs/skills #599 crosslinks
#   4. Retest unit + docs
#
# Optional chain: Playwright wrap-swap E9 (make setup-cloud-localterra first).
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
echo "  GitLab #599 — unwrap+≥2hop USTR→USTC gas combo"
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
    src/utils/constants.test.ts \
    src/components/common/__tests__/TerraClassicTxFeeHint.test.tsx \
    src/pages/SwapPage.test.tsx
}

run_docs() {
  set -euo pipefail
  rg -q 'UNWRAP_ROUTER_COMBO_OVERHEAD_GAS' frontend-dapp/src/utils/constants.ts
  rg -q 'unwrapRouterComboOverheadGas' frontend-dapp/src/services/terraclassic/terraGas.ts
  rg -q 'send_2hop_unwrap_ustc' frontend-dapp/src/services/terraclassic/terraGasRetailInventory.ts
  rg -q 'SendHookGasDecodeError' frontend-dapp/src/services/terraclassic/terraGas.ts
  rg -q 'swap-network-fee' frontend-dapp/src/pages/SwapPage.tsx
  rg -q '#599' docs/frontend.md
  rg -q 'UNWRAP_ROUTER_COMBO_OVERHEAD_GAS' docs/frontend.md
  rg -q '3,110,000' docs/frontend.md
  rg -q '#599' skills/AGENTS_TERRACLASSIC_GAS.md
  rg -q 'make verify-issue-599' skills/AGENTS_TERRACLASSIC_GAS.md
  rg -q '14c' skills/AGENTS_TERRACLASSIC_GAS.md
  rg -q 'AGENTS_TERRACLASSIC_GAS' AGENTS.md
  rg -q 'verify-issue-599' AGENTS.md
  rg -q 'verify-issue-599' docs/testing.md
  rg -q '#599' NATIVE_TOKEN_WRAPPING.md
  rg -q 'E9' frontend-dapp/e2e/wrap-swap.spec.ts
  rg -q 'never copy hybrid' frontend-dapp/src/services/terraclassic/router.ts
  rg -q '#599' skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md
}

echo ""
echo "── first pass ──"
run_step "frontend unit: unwrap+2hop envelope + Swap Network fee" \
  run_frontend_unit

run_step "docs: #599 + skill + crosslinks" \
  run_docs

echo ""
echo "── retest ──"
run_step "retest frontend unit" \
  run_frontend_unit

run_step "retest docs #599" \
  run_docs

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #599 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
