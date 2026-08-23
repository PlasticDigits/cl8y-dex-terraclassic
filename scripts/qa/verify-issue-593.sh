#!/usr/bin/env bash
# Automated verification for GitLab #593 — Create Token + manager console.
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
echo "  GitLab #593 — Create Token + manager console"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: SKU / invoice / extra-debit / pages / nav" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/communityTaxSku.test.ts \
    src/utils/communityTaxInvoice.test.ts \
    src/utils/taxPreviewMaxSpend.test.ts \
    src/utils/__tests__/maxSpendableAmount.test.ts \
    src/pages/CreateTokenPage.test.tsx \
    src/pages/ManageTokenPage.test.tsx \
    src/components/common/navItems.test.ts \
    src/services/terraclassic/__tests__/terraGas.retailShapes.test.ts'

run_step "code: pages import PayWithAnyToken (no router fork)" \
  bash -c 'grep -q "PayWithAnyToken" frontend-dapp/src/pages/CreateTokenPage.tsx && \
    grep -q "PayWithAnyToken" frontend-dapp/src/pages/ManageTokenPage.tsx && \
    ! grep -qE "executeMultiHopSwap|quoteCw20ViaRouteSolve" frontend-dapp/src/pages/CreateTokenPage.tsx && \
    ! grep -qE "executeMultiHopSwap|quoteCw20ViaRouteSolve" frontend-dapp/src/pages/ManageTokenPage.tsx'

run_step "code: extra-debit Max wired" \
  bash -c 'grep -q extraDebitSellBps frontend-dapp/src/pages/SwapPage.tsx && \
    grep -q extraDebitSellBps frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx'

run_step "docs: C593 + glossary + testing + skill" \
  bash -c 'grep -qE "C593-1" docs/frontend.md && \
    grep -q "Community tax token" docs/design-system.md && \
    grep -q "verify-issue-593" docs/testing.md && \
    grep -q "C593-1" skills/AGENTS_FRONTEND_CREATE_TOKEN.md && \
    grep -q "AGENTS_FRONTEND_CREATE_TOKEN" AGENTS.md'

run_step "launcher: 0-SKU CreateToken execute exists" \
  grep -q "CreateToken(Box<CreateTokenMsg>)" smartcontracts/contracts/community-token-launcher/src/msg.rs

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #593 verification passed"
