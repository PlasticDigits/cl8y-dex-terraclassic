#!/usr/bin/env bash
# Verification for Forgejo #1264 — USTC→USTR wrap+2hop stays pool-only at the #587 envelope.
#
# Does **not** raise WRAP_ROUTER_COMBO_OVERHEAD_GAS (columbus-5 AC1 still unmeasured).
# Proves:
#   1. wrap+2hop fixture = 2,710,000 (LUNC and USTC pay share the envelope)
#   2. Native wrap path never copies hybrid / book_input (H596-7 / #1280 AC4)
#   3. Docs/skills crosslinks
#
# Optional chain: wrap-swap E10 (VERIFY_ISSUE_1264_CHAIN=1).
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
echo "  Forgejo #1264 — USTC→USTR wrap+2hop pool-only envelope"
echo "════════════════════════════════════════════════════════════════"

run_frontend_unit() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/services/terraclassic/__tests__/terraGas.retailShapes.test.ts \
    src/services/terraclassic/__tests__/swapNetworkFee.test.ts \
    src/services/terraclassic/__tests__/terraGas.mixedHybrid.test.ts \
    src/utils/hybridHopOfferPartition.test.ts
}

run_docs() {
  set -euo pipefail
  rg -q 'wrap_plus_send_2hop_ustc' frontend-dapp/src/services/terraclassic/terraGasRetailInventory.ts
  rg -q 'WRAP_ROUTER_COMBO_OVERHEAD_GAS' frontend-dapp/src/utils/constants.ts
  rg -q 'never copy hybrid' frontend-dapp/src/services/terraclassic/router.ts
  rg -q 'E10:' frontend-dapp/e2e/wrap-swap.spec.ts
  rg -q '#1264' skills/AGENTS_TERRACLASSIC_GAS.md
  rg -q '#1264' docs/frontend.md
  rg -q 'verify-issue-1264' AGENTS.md
  rg -q 'verify-issue-1264' docs/testing.md
  rg -q 'H596-7' skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md
  rg -q '#1264' skills/AGENTS_HYBRID_HOP_OFFER.md
  rg -q '#1264' NATIVE_TOKEN_WRAPPING.md
}

echo ""
echo "── first pass ──"
run_step "frontend unit: wrap+2hop 2.71M + no hybrid on wrap hops" run_frontend_unit
run_step "docs: #1264 + E10 + H596-7" run_docs

echo ""
echo "── retest ──"
run_step "retest frontend unit" run_frontend_unit
run_step "retest docs" run_docs

if [ "${VERIFY_ISSUE_1264_CHAIN:-0}" = "1" ]; then
  if make has-localterra >/dev/null 2>&1 && [ -f frontend-dapp/.env.local ]; then
    run_step "chain: Playwright wrap-swap E10" \
      bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- \
        ./node_modules/.bin/playwright test e2e/wrap-swap.spec.ts --project=e2e-tx -g "E10"'
  else
    echo ""
    echo "[chain] SKIP — LocalTerra or frontend-dapp/.env.local not ready"
  fi
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> Forgejo #1264 verification passed (envelope unchanged; AC1 columbus-5 not claimed)"
