#!/usr/bin/env bash
# Automated verification for GitLab #528 — slippage preset chip alignment.
#
# Proves (unit + docs; Playwright P1–P10 when LocalTerra can boot the trade ticket):
#   1. Shared SlippageProtectionPresets: label above, 3-up group, Custom outside.
#   2. Trade Market + Swap Settings selection, default 5%, max_spread mapping.
#   3. Custom sanitize (A4) + high-warn >5% + range error <0.01.
#   4. Docs/skills invariants S528-1–S528-10 crosslinked.
#
# Refs: skills/AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md,
#       frontend-dapp/src/components/common/SlippageProtectionPresets.tsx,
#       docs/frontend.md § Slippage protection preset alignment
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
echo "  GitLab #528 — Slippage protection preset alignment"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: copy helpers + shared group + Trade + Swap Settings" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/slippageProtectionCopy.test.ts \
    src/components/common/__tests__/SlippageProtectionPresets.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.slippagePresets.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx \
    src/pages/SwapPage.test.tsx \
    src/components/swap/__tests__/SwapPreSubmitSummary.test.tsx'

run_step "code: Trade + Swap use SlippageProtectionPresets; no label+chip flex-wrap" \
  grep -qE 'SlippageProtectionPresets' frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx && \
  grep -qE 'SlippageProtectionPresets' frontend-dapp/src/pages/SwapPage.tsx && \
  grep -qE 'trade-market-slippage-presets' frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx && \
  grep -qE 'swap-slippage-presets' frontend-dapp/src/pages/SwapPage.tsx && \
  bash -c '! grep -nE "flex flex-wrap gap-2 text-\[10px\]" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx'

run_step "code: group is grid-cols-3; Custom outside group; Trade min-h-11" \
  grep -qE 'grid-cols-3' frontend-dapp/src/components/common/SlippageProtectionPresets.tsx && \
  grep -qE 'role="group"' frontend-dapp/src/components/common/SlippageProtectionPresets.tsx && \
  grep -qE 'customSlot' frontend-dapp/src/components/common/SlippageProtectionPresets.tsx && \
  grep -qE 'TRADE_SLIPPAGE_PRESET_CLASS' frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx && \
  grep -qE 'min-h-11' frontend-dapp/src/utils/tradeMoneyCta.ts && \
  grep -qE 'sanitizeSlippageCustomInput' frontend-dapp/src/pages/SwapPage.tsx

run_step "code: presets + default 5% + max_spread helper unchanged" \
  grep -qE 'SLIPPAGE_TOLERANCE_PRESETS_PERCENT = \[0.5, 1.0, 5.0\]' frontend-dapp/src/utils/slippageProtectionCopy.ts && \
  grep -qE 'DEFAULT_SLIPPAGE_TOLERANCE_PERCENT = 5' frontend-dapp/src/utils/slippageProtectionCopy.ts && \
  grep -qE 'maxSpreadFromSlippagePercent' frontend-dapp/src/utils/slippageProtectionCopy.ts && \
  bash -c '! grep -qE "dangerouslySetInnerHTML|innerHTML" frontend-dapp/src/components/common/SlippageProtectionPresets.tsx frontend-dapp/src/utils/slippageProtectionCopy.ts'

run_step "code: Pool withdraw 0.5/1.0/2.0 set untouched" \
  grep -qE "\['0.5', '1.0', '2.0'\]" frontend-dapp/src/components/pool/PoolAdvancedManage.tsx && \
  bash -c '! grep -qE "SlippageProtectionPresets" frontend-dapp/src/components/pool/PoolAdvancedManage.tsx'

run_step "docs: frontend.md S528-1–S528-10 + swap-max-spread invariant 10" \
  grep -qE 'slippage-protection-preset-align' docs/frontend.md && \
  grep -qE '\*\*S528-1\*\*' docs/frontend.md && \
  grep -qE '\*\*S528-10\*\*' docs/frontend.md && \
  grep -qE 'Preset chip grouping \(GitLab #528\)' docs/swap-max-spread-ux.md

run_step "skill: AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN + default-slippage crosslink" \
  grep -qE 'SlippageProtectionPresets' skills/AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md && \
  grep -qE 'make verify-issue-528' skills/AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md && \
  grep -qE 'S528-1' docs/frontend.md && \
  grep -qE 'AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN|#528' skills/AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md

run_step "crosslinks: onboarding + CTA dock + AGENTS.md" \
  grep -qE 'AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN|#528' skills/AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md && \
  grep -qE 'AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN|#528' skills/AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md && \
  grep -qE 'AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN|#528' AGENTS.md

if [[ "${VERIFY_ISSUE_528_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright P1–P10] skipped (VERIFY_ISSUE_528_SKIP_E2E=1)"
elif make has-localterra >/dev/null 2>&1; then
  # Host :1317 often hangs on Cloud Agent; Playwright globalSetup starts :13170.
  if curl -sf --connect-timeout 2 --max-time 5 http://127.0.0.1:13170/cosmos/base/tendermint/v1beta1/node_info >/dev/null; then
    export E2E_LCD_PROXY_URL="${E2E_LCD_PROXY_URL:-http://127.0.0.1:13170}"
    export VITE_TERRA_LCD_URL="${E2E_LCD_PROXY_URL}"
    export E2E_LCD_URL="${E2E_LCD_PROXY_URL}"
  fi
  run_step "playwright: P1–P10 chip geometry + Swap grouping" \
    bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test e2e/slippage-preset-align-528.spec.ts --project=e2e-smoke'
else
  echo ""
  echo "[playwright P1–P10] skipped — make has-localterra failed (probe ran; provision with make setup-cloud-localterra)"
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  Results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for line in "${RESULTS[@]}"; do
  echo "  $line"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
