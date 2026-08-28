#!/usr/bin/env bash
# Verification for GitLab #679 — mixed hybrid+pool router gas + Swap Network fee.
#
# Layers (no LocalTerra required for unit/docs):
#   1. Frontend unit: mixed-hop envelope, hint=broadcast, inventory, Swap RTL
#   2. Swarm gas.ts lockstep
#   3. Docs/skills #679 crosslinks
#   4. Child verifies: #475 inventory, #587 wrap+multihop, #596 no hybrid opt-out
#   5. Retest unit + docs
#
# Optional chain: VERIFY_ISSUE_679_CHAIN=1 (LocalTerra mixed-hop tx, 1 worker).
# columbus-5 4-hop anchor remains hash AB8BE4F7… (gas_used 5,026,176).
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
echo "  GitLab #679 — mixed hybrid router gas + Swap Network fee"
echo "════════════════════════════════════════════════════════════════"

run_frontend_unit() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/services/terraclassic/__tests__/terraGas.mixedHybrid.test.ts \
    src/services/terraclassic/__tests__/transactions.test.ts \
    src/services/terraclassic/__tests__/terraGas.retailShapes.test.ts \
    src/services/terraclassic/__tests__/swapNetworkFee.test.ts \
    src/services/terraclassic/__tests__/hybridSwapGas.test.ts \
    src/services/terraclassic/__tests__/terraWalletSignTxRaw.amino.test.ts \
    src/pages/SwapPage.test.tsx
}

bootstrap_swarm_worktree() {
  local swarm="packages/localnet-trading-swarm"
  if [[ -x "$swarm/node_modules/.bin/vitest" ]]; then
    return 0
  fi
  local common main_root
  common="$(git rev-parse --git-common-dir)"
  main_root="$(cd "$common/.." && pwd)"
  if [[ "$main_root" != "$REPO_ROOT" && -x "$main_root/$swarm/node_modules/.bin/vitest" ]]; then
    ln -sfn "$main_root/$swarm/node_modules" "$REPO_ROOT/$swarm/node_modules"
    echo "[bootstrap] linked $swarm/node_modules from primary checkout"
    return 0
  fi
  echo "[bootstrap] $swarm node_modules missing — npm ci…"
  bash "$REPO_ROOT/scripts/with-node.sh" --cwd "$swarm" -- npm ci
}

run_swarm_unit() {
  bootstrap_swarm_worktree
  bash scripts/with-node.sh --cwd packages/localnet-trading-swarm -- npm run test:run -- src/gas.test.ts
}

run_docs() {
  set -euo pipefail
  rg -q 'MIXED_HYBRID_ROUTER_HEADROOM_GAS' frontend-dapp/src/utils/constants.ts
  rg -q 'gasLimitForHybridRouterOperations' frontend-dapp/src/services/terraclassic/terraGas.ts
  rg -q 'send_4hop_hybrid_first_pool_rest' frontend-dapp/src/services/terraclassic/terraGasRetailInventory.ts
  rg -q 'cw20RouterOperations' frontend-dapp/src/services/terraclassic/swapNetworkFee.ts
  rg -q 'swap-wallet-fee-note' frontend-dapp/src/pages/SwapPage.tsx
  rg -q '#679' docs/frontend.md
  rg -q 'MIXED_HYBRID_ROUTER_HEADROOM_GAS' docs/frontend.md
  rg -q '5,026,176' docs/frontend.md
  rg -q '#679' skills/AGENTS_TERRACLASSIC_GAS.md
  rg -q 'make verify-issue-679' skills/AGENTS_TERRACLASSIC_GAS.md
  rg -q 'G-AUTO-1' skills/AGENTS_TERRACLASSIC_GAS.md
  rg -q '#679' skills/AGENTS_FRONTEND_STATION_SIGNING.md
  rg -q 'verify-issue-679' AGENTS.md
  rg -q 'verify-issue-679' docs/testing.md
  rg -q '#679' docs/user-incident-faq.md
  rg -q 'MIXED_HYBRID_ROUTER_HEADROOM_GAS' packages/localnet-trading-swarm/src/gas.ts
  rg -q 'isAtomicWalletConnectPost' frontend-dapp/src/services/terraclassic/terraWalletSignTxRaw.ts
}

echo ""
echo "── first pass ──"
run_step "frontend unit: mixed-hop envelope + Swap Network fee" \
  run_frontend_unit

run_step "swarm unit: mixed-hop lockstep" \
  run_swarm_unit

run_step "docs: #679 + skill + crosslinks" \
  run_docs

run_step "child: verify-issue-475 inventory" \
  make verify-issue-475

run_step "child: verify-issue-587 wrap+multihop unchanged" \
  make verify-issue-587

run_step "child: verify-issue-596 no hybrid opt-out" \
  make verify-issue-596

echo ""
echo "── retest ──"
run_step "retest frontend unit" \
  run_frontend_unit

run_step "retest swarm unit" \
  run_swarm_unit

run_step "retest docs #679" \
  run_docs

if [[ "${VERIFY_ISSUE_679_CHAIN:-}" == "1" ]]; then
  echo ""
  echo "── optional chain (VERIFY_ISSUE_679_CHAIN=1) ──"
  echo "  No dedicated Playwright mixed-hop spec. Use Keplr/simulated wallet"
  echo "  (not Station) on LocalTerra for a ≥2-hop CW20 path with hybrid on"
  echo "  one hop only. 4-hop columbus-5 anchor: AB8BE4F7… gas_used 5,026,176."
  ok "chain note (2-hop LocalTerra stand-in; 4-hop hash is the anchor)"
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #679 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
