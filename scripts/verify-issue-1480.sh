#!/usr/bin/env bash
# Exact offline shape and fee parity; funded/physical wallet acceptance is separate.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
ISSUE=1480
source scripts/qa/verify-followup-common.sh
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/services/terraclassic/__tests__/offlineSwapBudget.test.ts \
    src/services/terraclassic/__tests__/signedOfflineSwapFee.test.ts \
    src/services/terraclassic/__tests__/terraGas.retailShapes.test.ts \
    src/services/terraclassic/__tests__/terraGas.mixedHybrid.test.ts \
    src/services/terraclassic/__tests__/swapNetworkFee.test.ts \
    src/services/terraclassic/__tests__/terraBroadcast.autoGas.test.ts \
    src/services/terraclassic/__tests__/terraBroadcastRecovery.test.ts \
    src/services/terraclassic/__tests__/terraWalletSignLock.test.ts \
    src/utils/__tests__/terraAccountSequence.test.ts \
    src/utils/__tests__/humanizeTerraTxError.test.ts \
    src/utils/swapRouteSlippage.test.ts \
    src/components/swap/__tests__/ExpertModeModal.test.tsx \
    src/components/swap/ExpertModeModal.test.tsx \
    src/services/terraclassic/__tests__/swapAutoGasFeeHint.test.tsx \
    src/utils/__tests__/swapNativeGasBalanceGate.test.ts \
    src/utils/__tests__/maxSpendableAmount.test.ts \
    src/pages/SwapPage.test.tsx src/pages/TradePage.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.slippagePresets.test.tsx
}
funded() {
  python3 scripts/qa/localterra-offline-gas-1480.py --container "$VERIFY_ISSUE_1480_CONTAINER" \
    --artifacts "$VERIFY_ISSUE_1480_ARTIFACTS" --output "$VERIFY_ISSUE_1480_RECEIPT_PREFIX-$1.json"
}
start
for attempt in first retest; do
  run "$attempt actual decoded fee and submitted-message guard" python3 scripts/qa/test-localterra-offline-gas-1480.py
  selected "$attempt actual entry, signed fee, failure and balance regressions" FRONTEND frontend
  if [[ -n "${VERIFY_ISSUE_1480_CONTAINER:-}" && -n "${VERIFY_ISSUE_1480_ARTIFACTS:-}" && -n "${VERIFY_ISSUE_1480_RECEIPT_PREFIX:-}" ]]; then
    run "$attempt owned funded current-artifact route budgets" funded "$attempt"
  else skip "$attempt owned funded chain: configure explicit owned container, artifacts and receipt prefix; no shared chain reset"; fi
done
selected 'frontend types' TYPES bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/tsc -b --pretty false
run 'verifier syntax' bash -n scripts/verify-issue-1480.sh scripts/qa/verify-followup-common.sh
run 'funded driver syntax' python3 -c "import ast,pathlib; ast.parse(pathlib.Path('scripts/qa/localterra-offline-gas-1480.py').read_text())"
skip 'Physical wallet requested-versus-signed gas, native/tax/hook/populated-book worst case and production settlement require actual receipts'
run 'invariant and checklist crosslinks' documentation
finish
