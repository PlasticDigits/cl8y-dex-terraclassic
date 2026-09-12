#!/usr/bin/env bash
# Verification for Forgejo #1222 — retail gas envelope vs wallet auto-fee census.
#
# Docs / process only (no LocalTerra, no envelope retune):
#   G-CENSUS-1  one ADR covering hint vs broadcast vs wallet vs measured usage
#   G-CENSUS-2  dated measurements or explicit unmeasured rows
#   G-CENSUS-3  G0–G7 scored; Stay vs spawn
#   G-CENSUS-4  #123 / #546/#618 / #1209 out of census
#   G-CENSUS-5  no production gas-constant or wallet-adapter diff
#   G-CENSUS-6  no impl spawn unless decision is not Stay
#   G-CENSUS-7  no LCD-sim default / no hybrid-off
#   G-CENSUS-8  uluna-only fee; no mainnet SEC-E08 from this census
#
# Refs: docs/adr/0004-terraclassic-retail-gas-census.md
#       skills/AGENTS_TERRACLASSIC_GAS.md
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

ADR="docs/adr/0004-terraclassic-retail-gas-census.md"

PROD_GAS_FILES=(
  frontend-dapp/src/services/terraclassic/terraGas.ts
  frontend-dapp/src/services/terraclassic/hybridSwapGas.ts
  frontend-dapp/src/services/terraclassic/swapNetworkFee.ts
  frontend-dapp/src/services/terraclassic/terraClassicFeeEstimate.ts
  frontend-dapp/src/services/terraclassic/terraBroadcast.ts
  frontend-dapp/src/services/terraclassic/terraWalletSignTxRaw.ts
  frontend-dapp/src/services/terraclassic/stationExtensionConfig.ts
  frontend-dapp/src/utils/extensionSignedFeeGuard.ts
  frontend-dapp/src/utils/constants.ts
)

echo "════════════════════════════════════════════════════════════════"
echo "  Forgejo #1222 — Terra Classic retail gas census"
echo "════════════════════════════════════════════════════════════════"

run_adr_headings() {
  test -f "$ADR"
  grep -qE '^## Status' "$ADR"
  grep -qE '^## Context' "$ADR"
  grep -qE '^## Decision' "$ADR"
  grep -qE '^## Invariants \(G-CENSUS\)' "$ADR"
  grep -qE '^## Retail execute inventory vs hint vs wallet' "$ADR"
  grep -qE '^## Dated measurements' "$ADR"
  grep -qE '^## Wallet classes' "$ADR"
  grep -qE '^## Attack / abuse evaluation' "$ADR"
  grep -qE '^## Test plan \(research paths\)' "$ADR"
  grep -qE '\*\*Stay\*\*' "$ADR"
  grep -qE '\*\*No impl spawn\*\*' "$ADR"
  grep -qE '\*\*G0\*\*' "$ADR"
  grep -qE '\*\*G7\*\*' "$ADR"
  grep -qE '\*\*G-CENSUS-1\*\*' "$ADR"
  grep -qE '\*\*G-CENSUS-8\*\*' "$ADR"
  grep -qE '\*\*A1\*\*' "$ADR"
  grep -qE '\*\*A6\*\*' "$ADR"
  grep -qE 'issues/123' "$ADR"
  grep -qE 'issues/546' "$ADR"
  grep -qE 'issues/618' "$ADR"
  grep -qE 'issues/1209' "$ADR"
  grep -qE 'AB8BE4F75E051837BB01C364DEDE6611727E47F0F857AADF04B17C39F360446D' "$ADR"
  grep -qE '5,026,176' "$ADR"
  grep -qE '6,785,500' "$ADR"
  grep -qE '30121174' "$ADR"
  grep -qE '2026-09-11' "$ADR"
  grep -qE 'unmeasured' "$ADR"
  grep -qE 'uluna' "$ADR"
  grep -qE 'preferNoSetFee' "$ADR"
  grep -qE 'isAtomicWalletConnectPost' "$ADR"
  grep -qE 'LCD simulate is \*\*not\*\* the production limiter' "$ADR"
  grep -qE 'H596' "$ADR"
  grep -qE 'SEC-E08' "$ADR"
  grep -qE 'make verify-issue-1222' "$ADR"
}

run_no_lcd_sim_or_hybrid_off() {
  grep -qE 'Recommending LCD-sim-only' "$ADR"
  grep -qE 'Recommending pool-only as the gas fix' "$ADR"
  grep -qE 'Suggesting `uusd` fees' "$ADR"
  grep -qE 'Making simulate the production limiter' "$ADR"
  grep -qE '\*\*Stay\*\*' "$ADR"
  grep -qE '\*\*No impl spawn\*\*' "$ADR"
  grep -qE 'LCD simulate is \*\*not\*\* the production limiter' "$ADR"
}

run_docs_crosslinks() {
  grep -qE '0004-terraclassic-retail-gas-census' docs/README.md
  grep -qE 'verify-issue-1222' docs/testing.md
  grep -qE 'G-CENSUS-1' docs/testing.md
  grep -qE 'ADR 0004|#1222' docs/frontend.md
  grep -qE '0004-terraclassic-retail-gas-census' docs/frontend.md
  grep -qE 'G-CENSUS-1' skills/AGENTS_TERRACLASSIC_GAS.md
  grep -qE 'G-CENSUS-8' skills/AGENTS_TERRACLASSIC_GAS.md
  grep -qE 'make verify-issue-1222' skills/AGENTS_TERRACLASSIC_GAS.md
  grep -qE '0004-terraclassic-retail-gas-census' skills/AGENTS_TERRACLASSIC_GAS.md
  grep -qE '#1222|0004-terraclassic-retail-gas-census' skills/AGENTS_FRONTEND_STATION_SIGNING.md
  grep -qE 'verify-issue-1222' AGENTS.md
  grep -qE 'G-CENSUS-1' AGENTS.md
  grep -qE 'G-CENSUS-1' docs/qa/issue-1222/README.md
  grep -qE 'does \*\*not\*\* retune|docs-only' docs/qa/issue-1222/README.md
}

run_constants_lockstep() {
  grep -qE 'export const WRAP_GAS_LIMIT = 400000' frontend-dapp/src/utils/constants.ts
  grep -qE 'export const UNWRAP_GAS_LIMIT = 800_000' frontend-dapp/src/utils/constants.ts
  grep -qE 'export const WRAP_ROUTER_COMBO_OVERHEAD_GAS = 400_000' frontend-dapp/src/utils/constants.ts
  grep -qE 'export const UNWRAP_ROUTER_COMBO_OVERHEAD_GAS = 400_000' frontend-dapp/src/utils/constants.ts
  grep -qE 'export const MIXED_HYBRID_ROUTER_HEADROOM_GAS = 2_150_000' frontend-dapp/src/utils/constants.ts
  grep -qE 'export const MIN_GAS_PRICE_ULUNA = 28.325' frontend-dapp/src/utils/constants.ts
  grep -qE 'export const BASE_GAS_LIMIT = 200000' frontend-dapp/src/services/terraclassic/terraGas.ts
  grep -qE 'export const HYBRID_SWAP_GAS_LIMIT = 15_000_000' frontend-dapp/src/services/terraclassic/hybridSwapGas.ts
  grep -qE 'send_4hop_hybrid_first_pool_rest' frontend-dapp/src/services/terraclassic/terraGasRetailInventory.ts
  grep -qE 'cw20RouterOperations' frontend-dapp/src/services/terraclassic/swapNetworkFee.ts
  grep -qE 'isAtomicWalletConnectPost' frontend-dapp/src/services/terraclassic/terraWalletSignTxRaw.ts
}

run_no_prod_gas_diff() {
  local base=""
  if git rev-parse --verify origin/main >/dev/null 2>&1; then
    base="$(git merge-base HEAD origin/main)"
  elif git rev-parse --verify main >/dev/null 2>&1; then
    base="$(git merge-base HEAD main)"
  fi
  if [[ -z "$base" ]]; then
    echo "no origin/main merge-base — skip git-diff gate (constants lockstep still runs)"
    return 0
  fi
  local dirty
  dirty="$(git diff --name-only "$base" -- "${PROD_GAS_FILES[@]}" || true)"
  if [[ -n "$dirty" ]]; then
    echo "production gas/wallet files changed vs merge-base:" >&2
    echo "$dirty" >&2
    return 1
  fi
}

run_with_node() {
  mkdir -p "${NVM_DIR:-$HOME/.nvm}"
  if bash "$REPO_ROOT/scripts/with-node.sh" --cwd frontend-dapp -- "$@"; then
    return 0
  fi
  local major
  major="$(node -v 2>/dev/null | sed 's/^v//' | cut -d. -f1 || true)"
  if [[ "${major:-0}" -ge 24 ]]; then
    echo "[bootstrap] with-node.sh unavailable; PATH node $(node -v)"
    (cd frontend-dapp && "$@")
    return $?
  fi
  echo "need Node 24+ (nvm or PATH)" >&2
  return 1
}

run_frontend_unit() {
  if [[ ! -x frontend-dapp/node_modules/.bin/vitest ]]; then
    echo "[bootstrap] frontend-dapp node_modules missing — npm ci…"
    run_with_node npm ci
  fi
  run_with_node npm run test:run -- \
    src/services/terraclassic/__tests__/terraGas.mixedHybrid.test.ts \
    src/services/terraclassic/__tests__/swapNetworkFee.test.ts \
    src/services/terraclassic/__tests__/terraGas.retailShapes.test.ts
}

echo ""
echo "── first pass ──"
run_step "ADR: required headings + G0–G7 + Stay + measurements" \
  run_adr_headings

run_step "ADR: attack rows do not recommend LCD-sim / hybrid-off / uusd" \
  run_no_lcd_sim_or_hybrid_off

run_step "docs: README + testing + frontend + skills + AGENTS.md + QA" \
  run_docs_crosslinks

run_step "lockstep: current envelopes still match census numbers" \
  run_constants_lockstep

run_step "AC5: production gas/wallet files untouched vs merge-base" \
  run_no_prod_gas_diff

run_step "unit: mixed hybrid + Network fee + retail inventory (current code)" \
  run_frontend_unit

echo ""
echo "── retest ──"
run_step "retest ADR headings" \
  run_adr_headings

run_step "retest AC5 production files untouched" \
  run_no_prod_gas_diff

run_step "retest unit lockstep" \
  run_frontend_unit

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  $PASS passed, $FAIL failed"
echo "────────────────────────────────────────────────────────────────"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
