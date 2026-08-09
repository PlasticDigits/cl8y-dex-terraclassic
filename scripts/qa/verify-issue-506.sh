#!/usr/bin/env bash
# Verification for GitLab #506 — /ust1 oracle window UI (client math, gates, gas, nav).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0

ok()  { PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #506 — UST1 window UI"
echo "════════════════════════════════════════════════════════════════"

echo ""
echo "[1] Frontend unit tests (math, gates, client, page, nav, retail gas)..."
if bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
  src/utils/__tests__/ust1WindowMath.test.ts \
  src/utils/__tests__/ust1WindowGates.test.ts \
  src/services/terraclassic/__tests__/ust1Window.test.ts \
  src/pages/Ust1Page.test.tsx \
  src/components/common/navItems.test.ts \
  src/services/terraclassic/__tests__/terraGas.retailShapes.test.ts
then
  ok "unit tests"
else
  bad "unit tests"
fi

echo ""
echo "[2] Source guards (route + label + no faucet overload)..."
if rg -q "path=\"/ust1\"" frontend-dapp/src/App.tsx \
  && rg -q "UST1_NAV_ITEM" frontend-dapp/src/components/common/navItems.ts \
  && rg -q "label: 'UST1'" frontend-dapp/src/components/common/navItems.ts \
  && ! rg -n "label: 'Mint'" frontend-dapp/src/components/common/navItems.ts | rg -q ust1
then
  ok "route/nav labels"
else
  bad "route/nav labels"
fi

if rg -q "UST1_WINDOW_SEND_GAS_LIMIT" frontend-dapp/src/services/terraclassic/terraGas.ts \
  && rg -q "send_inner_ust1_deposit" frontend-dapp/src/services/terraclassic/terraGasRetailInventory.ts
then
  ok "gas inventory for deposit/withdraw send"
else
  bad "gas inventory for deposit/withdraw send"
fi

if rg -q "VITE_UST1_WINDOW_ADDRESS" docker/frontend/Dockerfile \
  && rg -q "VITE_UST1_WINDOW_ADDRESS" frontend-dapp/.env.example
then
  ok "Coolify / env examples"
else
  bad "Coolify / env examples"
fi

if test -f docs/runbooks/ust1-window-ui.md && test -f skills/AGENTS_UST1_WINDOW_UI.md; then
  ok "runbook + agent skill"
else
  bad "runbook + agent skill"
fi

if rg -q "TokenLogo" frontend-dapp/src/pages/Ust1Page.tsx \
  && rg -q "ust1-withdraw-slippage-note" frontend-dapp/src/pages/Ust1Page.tsx \
  && rg -q "TxResultAlert" frontend-dapp/src/pages/Ust1Page.tsx
then
  ok "page logos + slippage disclosure + explorer success"
else
  bad "page logos + slippage disclosure + explorer success"
fi

if test -f frontend-dapp/e2e/ust1-window.spec.ts \
  && test -f frontend-dapp/e2e/helpers/ust1-window-lcd-mock.ts
then
  ok "Playwright ust1-window gate specs"
else
  bad "Playwright ust1-window gate specs"
fi

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  PASS=$PASS  FAIL=$FAIL"
echo "────────────────────────────────────────────────────────────────"
if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
echo "All #506 checks passed."
