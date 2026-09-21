#!/usr/bin/env bash
# Automated verification for Forgejo #1255 — Swap token_info decimals for unlisted CW20s.
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
echo "  Forgejo #1255 — Swap unlisted CW20 amount scale"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: resolver / hook / cache / Swap RTL 18-dec vs 6-dec" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/swapAssetDecimals.test.ts \
    src/hooks/useAssetDecimals.test.tsx \
    src/pages/SwapPage.assetDecimals.test.tsx \
    src/utils/__tests__/tokenDisplay.test.ts \
    src/utils/swapQuoteAmountScale.test.ts \
    src/utils/swapDisclosure.test.ts'

run_step "code: Swap/Trade execute use useAssetDecimals (not getDecimals)" \
  bash -c '
    set -euo pipefail
    grep -q "useAssetDecimals" frontend-dapp/src/pages/SwapPage.tsx
    grep -q "useAssetDecimals" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx
    grep -q "resolveSwapAssetDecimals" frontend-dapp/src/utils/swapAssetDecimals.ts
    grep -q "cl8y-dex-token-info-v2" frontend-dapp/src/utils/tokenDisplay.ts
    ! grep -q "getDecimals" frontend-dapp/src/pages/SwapPage.tsx
    ! grep -q "getDecimals" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx
    grep -q "payDecimals === null" frontend-dapp/src/utils/swapDisclosure.ts
  '

run_step "code: hostile parse + LCD only when lcdEnabled" \
  bash -c '
    set -euo pipefail
    grep -q "parseTokenInfoDecimals" frontend-dapp/src/utils/swapAssetDecimals.ts
    grep -q "lcdEnabled" frontend-dapp/src/hooks/useAssetDecimals.ts
    grep -q "getAllTokens" frontend-dapp/src/pages/SwapPage.tsx
    grep -q "decimalsHostile" frontend-dapp/src/utils/tokenDisplay.ts
  '

run_step "docs: Q1255 + skill + testing + AGENTS" \
  bash -c '
    set -euo pipefail
    grep -q "swap-amount-scaling" docs/frontend.md
    grep -qE "\*\*Q1255-1\*\*" docs/frontend.md
    grep -qE "\*\*Q1255-8\*\*" docs/frontend.md
    grep -q "verify-issue-1255" docs/testing.md
    grep -q "Q1255-1" skills/AGENTS_FRONTEND_SWAP_AMOUNT_SCALE.md
    grep -q "AGENTS_FRONTEND_SWAP_AMOUNT_SCALE" AGENTS.md
    grep -q "Q1255-1" docs/contracts-security-audit.md
    grep -q "token_info.decimals" skills/AGENTS_FRONTEND_SWAP_TOKENLIST_SYMBOLS.md
    grep -q "useAssetDecimals" skills/AGENTS_FRONTEND_SWAP_USTR_USDT_SCALE.md
  '

run_step "skill: AGENTS_FRONTEND_SWAP_AMOUNT_SCALE exists" \
  test -f skills/AGENTS_FRONTEND_SWAP_AMOUNT_SCALE.md

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> Forgejo #1255 verification passed"
