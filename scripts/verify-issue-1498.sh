#!/usr/bin/env bash
# Source/component geometry evidence is separate from production settlement.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
ISSUE=1498
source scripts/qa/verify-followup-common.sh
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/vitest run \
    src/pages/SwapPage.test.tsx src/pages/SwapPage.acquireGuidance.test.tsx \
    src/pages/SwapPage.queryParams.test.tsx src/pages/SwapPage.extraDebitSell.test.tsx \
    src/pages/SwapPage.feeDiscountRegistryBanner.test.tsx \
    src/components/swap/__tests__/SwapPreSubmitSummary.test.tsx \
    src/components/swap/SwapQuoteSummary.test.tsx \
    src/services/terraclassic/__tests__/swapAutoGasFeeHint.test.tsx
}
browser() {
  LOCAL_QUOTE_BROWSER_GEOMETRY_ONLY=1 \
  LOCAL_QUOTE_BROWSER_RECEIPT="${VERIFY_ISSUE_1498_RECEIPT_DIR:-/tmp}/swap-layout-1498-$1.json" \
  bash scripts/with-node.sh -- node scripts/qa/browser-local-quotes-1452.cjs
}
start
for attempt in first retest; do
  selected "$attempt layout, disconnected minimum and signing guards" FRONTEND frontend
  selected "$attempt production-build desktop/phone geometry and keyboard" BROWSER browser "$attempt"
done
selected 'frontend types' TYPES bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/tsc -b --pretty false
run 'verifier syntax' bash -n scripts/verify-issue-1498.sh
run 'layout invariant crosslinks' python3 -c "from pathlib import Path; s=Path('skills/AGENTS_SWAP_LAYOUT.md').read_text(); assert all(f'SL1498-{i}' in s for i in range(1,11)); assert 'issue-1498/README.md' in s; assert 'AGENTS_SWAP_LAYOUT.md' in Path('AGENTS.md').read_text()"
skip 'Live artifact/device/funded acceptance requires separate real receipts'
finish
