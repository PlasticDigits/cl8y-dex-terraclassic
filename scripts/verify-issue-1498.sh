#!/usr/bin/env bash
# Source/component geometry evidence is separate from production settlement.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
ISSUE=1498
source scripts/qa/verify-followup-common.sh
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/vitest run \
    src/pages/SwapPage.test.tsx src/pages/SwapPage.acquireGuidance.test.tsx \
    src/components/ui/__tests__/Modal.test.tsx \
    src/pages/SwapPage.queryParams.test.tsx src/pages/SwapPage.extraDebitSell.test.tsx \
    src/pages/SwapPage.feeDiscountRegistryBanner.test.tsx \
    src/components/swap/__tests__/SwapPreSubmitSummary.test.tsx \
    src/components/swap/SwapQuoteSummary.test.tsx \
    src/services/terraclassic/__tests__/swapAutoGasFeeHint.test.tsx
}
browser() {
  local attempt="$1" viewport fixtureBuild browserPort providerPort
  fixtureBuild="$(mktemp -d /tmp/swap-layout-1498-build.XXXXXX)"
  read -r browserPort providerPort < <(python3 - <<'PYPORTS'
import socket
with socket.socket() as api, socket.socket() as provider:
    api.bind(('127.0.0.1', 0)); provider.bind(('127.0.0.1', 0))
    print(api.getsockname()[1], provider.getsockname()[1])
PYPORTS
  )
  local -a fixtureEnv=(LOCAL_QUOTE_BROWSER_BUILD_DIR="$fixtureBuild" LOCAL_QUOTE_BROWSER_PORT="$browserPort" LOCAL_QUOTE_BROWSER_PROVIDER_PORT="$providerPort")
  # Compile once outside bounded browser test processes; subsequent batches
  # reject a stale source hash or mismatched controlled endpoint configuration.
  env "${fixtureEnv[@]}" LOCAL_QUOTE_BROWSER_BUILD_ONLY=1 \
    bash scripts/with-node.sh -- node scripts/qa/browser-local-quotes-1452.cjs || return
  # Each browser batch owns an isolated fixture and has a hard sub-minute cap.
  # Short-screen CTA bounds/hit tests are independent of the tall desktop control.
  for viewport in 1440x1000 375x667; do
    timeout 55s env "${fixtureEnv[@]}" LOCAL_QUOTE_BROWSER_SKIP_BUILD=1 LOCAL_QUOTE_BROWSER_GEOMETRY_ONLY=1 \
      SWAP_GEOMETRY_VIEWPORTS="$viewport" \
      LOCAL_QUOTE_BROWSER_RECEIPT="${VERIFY_ISSUE_1498_RECEIPT_DIR:-/tmp}/swap-layout-1498-$attempt-$viewport.json" \
      bash scripts/with-node.sh -- node scripts/qa/browser-local-quotes-1452.cjs || return
  done
  timeout 55s env "${fixtureEnv[@]}" LOCAL_QUOTE_BROWSER_SKIP_BUILD=1 LOCAL_QUOTE_BROWSER_GEOMETRY_ONLY=1 \
    SWAP_GEOMETRY_COMPACT_ONLY=1 SWAP_GEOMETRY_VIEWPORTS=320x568,390x844,414x896,430x932,667x375,767x900,768x900 \
    LOCAL_QUOTE_BROWSER_RECEIPT="${VERIFY_ISSUE_1498_RECEIPT_DIR:-/tmp}/swap-layout-1498-$attempt-compact.json" \
    bash scripts/with-node.sh -- node scripts/qa/browser-local-quotes-1452.cjs || return
  local connection theme
  for connection in 0 1; do
    theme=dark
    if [[ "$connection" == 1 ]]; then theme=light; fi
    timeout 55s env "${fixtureEnv[@]}" LOCAL_QUOTE_BROWSER_SKIP_BUILD=1 LOCAL_QUOTE_BROWSER_GEOMETRY_ONLY=1 \
      SWAP_GEOMETRY_COMPACT_ONLY=1 SWAP_GEOMETRY_AXE=1 SWAP_GEOMETRY_CONNECTED="$connection" SWAP_GEOMETRY_THEME="$theme" SWAP_GEOMETRY_VIEWPORTS=375x667 \
      LOCAL_QUOTE_BROWSER_RECEIPT="${VERIFY_ISSUE_1498_RECEIPT_DIR:-/tmp}/swap-layout-1498-$attempt-connected-$connection.json" \
      bash scripts/with-node.sh -- node scripts/qa/browser-local-quotes-1452.cjs || return
  done
  timeout 55s env "${fixtureEnv[@]}" LOCAL_QUOTE_BROWSER_SKIP_BUILD=1 LOCAL_QUOTE_BROWSER_GEOMETRY_ONLY=1 \
    SWAP_GEOMETRY_REVIEW_ONLY=1 SWAP_GEOMETRY_CONNECTED=1 SWAP_GEOMETRY_VIEWPORTS=375x667 \
    LOCAL_QUOTE_BROWSER_RECEIPT="${VERIFY_ISSUE_1498_RECEIPT_DIR:-/tmp}/swap-layout-1498-$attempt-review.json" \
    bash scripts/with-node.sh -- node scripts/qa/browser-local-quotes-1452.cjs || return
  rm -rf "$fixtureBuild"
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
