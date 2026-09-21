#!/usr/bin/env bash
# Verification for Forgejo #1264 — USTC→USTR wrap+2hop stays pool-only at the #587 envelope.
#
# Does **not** raise WRAP_ROUTER_COMBO_OVERHEAD_GAS (AC1 measured not-A/not-B;
# envelope stays 2,710,000). Do **not** treat AC1 as unmeasured.
# Optional columbus-5 LCD: VERIFY1264_COLUMBUS_TX=<hash>
# Known AC1: 53B06B653D78AF3683F51A065FC640074A3A2E76CAE343A97B3E0F0BD79BAC36
#   gas_wanted=2710000 gas_used=2630228 class not-A/not-B.
# Proves:
#   1. wrap+2hop fixture = 2,710,000 (LUNC and USTC pay share the envelope)
#   2. Native wrap path never copies hybrid / book_input (H596-7 / #1280 AC4)
#   3. USTC Max / gas-gate use bank LUNC, not micro-uusd (G1264-4)
#   4. Docs/skills crosslinks
#
# Optional chain: wrap-swap E10 (VERIFY_ISSUE_1264_CHAIN=1 or VERIFY1264_REQUIRE_CHAIN=1).
# VERIFY1264_REQUIRE_MAINNET=1 — FAIL when the hash is unset / LCD miss.
# Pin the known AC1 hash with VERIFY1264_COLUMBUS_TX (do not treat AC1 as open).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }
skip() { RESULTS+=("SKIP  $1"); echo "  [SKIP] $1"; }

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

export PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-31600}"
COLUMBUS_LCD="${VERIFY1264_COLUMBUS_LCD:-https://terra-classic-lcd.publicnode.com}"
WRAP_2HOP_ENVELOPE=2710000

run_frontend_unit() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/services/terraclassic/__tests__/terraGas.retailShapes.test.ts \
    src/services/terraclassic/__tests__/swapNetworkFee.test.ts \
    src/services/terraclassic/__tests__/terraGas.mixedHybrid.test.ts \
    src/utils/hybridHopOfferPartition.test.ts \
    src/utils/nativeWrapSwapHints.test.ts \
    src/utils/__tests__/swapNativeGasBalanceGate.test.ts \
    src/utils/__tests__/maxSpendableAmount.test.ts
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/pages/SwapPage.test.tsx -t "#1264"
}

run_docs() {
  set -euo pipefail
  rg -q 'wrap_plus_send_2hop_ustc' frontend-dapp/src/services/terraclassic/terraGasRetailInventory.ts
  rg -q 'WRAP_ROUTER_COMBO_OVERHEAD_GAS = 400_000' frontend-dapp/src/utils/constants.ts
  rg -q 'never copy hybrid' frontend-dapp/src/services/terraclassic/router.ts
  rg -q 'E10:' frontend-dapp/e2e/wrap-swap.spec.ts
  rg -q '#1264' skills/AGENTS_TERRACLASSIC_GAS.md
  rg -q 'G1264-1' skills/AGENTS_TERRACLASSIC_GAS.md
  rg -q 'G1264-8' skills/AGENTS_TERRACLASSIC_GAS.md
  rg -q '#1264' docs/frontend.md
  rg -q 'G1264-4' docs/frontend.md
  rg -q 'verify-issue-1264' AGENTS.md
  rg -q 'verify-issue-1264' docs/testing.md
  rg -q 'H596-7' skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md
  rg -q '#1264' skills/AGENTS_HYBRID_HOP_OFFER.md
  rg -q '#1264' NATIVE_TOKEN_WRAPPING.md
  rg -q 'VERIFY1264_COLUMBUS_TX' scripts/qa/verify-issue-1264.sh
  rg -q 'G1264' docs/qa-invariants.md
  rg -q 'isNativeUlunaDenom' frontend-dapp/src/pages/SwapPage.tsx
  rg -q 'defaultNativeWrapHopCount' frontend-dapp/src/pages/SwapPage.tsx
  # Do not raise the combo in this ticket — AC1 measured not-A; envelope stays.
  if rg -n 'WRAP_ROUTER_COMBO_OVERHEAD_GAS =' frontend-dapp/src/utils/constants.ts | rg -v '400_000'; then
    echo "WRAP_ROUTER_COMBO_OVERHEAD_GAS must stay 400_000 (AC1 not-A; do not raise)" >&2
    exit 1
  fi
}

echo ""
echo "── first pass ──"
run_step "frontend unit: wrap+2hop 2.71M + USTC LUNC gate + no hybrid on wrap hops" run_frontend_unit
run_step "docs: #1264 + E10 + H596-7 + G1264" run_docs

echo ""
echo "── retest ──"
run_step "retest frontend unit" run_frontend_unit
run_step "retest docs" run_docs

want_chain=0
if [ "${VERIFY_ISSUE_1264_CHAIN:-0}" = "1" ] || [ "${VERIFY1264_REQUIRE_CHAIN:-0}" = "1" ]; then
  want_chain=1
fi

if [ "$want_chain" = "1" ]; then
  if make has-localterra >/dev/null 2>&1; then
    if [[ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]]; then
      COMMON="$(git rev-parse --git-common-dir)"
      MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
      if [[ -f "$MAIN_ROOT/frontend-dapp/.env.local" && "$MAIN_ROOT" != "$REPO_ROOT" ]]; then
        cp "$MAIN_ROOT/frontend-dapp/.env.local" "$REPO_ROOT/frontend-dapp/.env.local"
      fi
    fi
    if [[ -f "$REPO_ROOT/frontend-dapp/.env.local" && -x "$REPO_ROOT/frontend-dapp/node_modules/.bin/playwright" ]]; then
      run_step "chain: Playwright wrap-swap E10" \
        bash -c 'CI=1 PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-31600}" PLAYWRIGHT_BASE_URL="http://127.0.0.1:${PLAYWRIGHT_WEB_PORT:-31600}" bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-tx --workers=1 e2e/wrap-swap.spec.ts -g "E10"'
    else
      if [ "${VERIFY1264_REQUIRE_CHAIN:-0}" = "1" ]; then
        echo ""
        echo "[chain]"
        bad "Playwright wrap-swap E10 (missing .env.local or Playwright)"
      else
        skip "chain: Playwright wrap-swap E10 (missing .env.local or Playwright)"
      fi
    fi
  else
    if [ "${VERIFY1264_REQUIRE_CHAIN:-0}" = "1" ]; then
      echo ""
      echo "[chain]"
      bad "LocalTerra required (VERIFY1264_REQUIRE_CHAIN=1) — make setup-cloud-localterra"
    else
      echo ""
      echo "[chain] SKIP — LocalTerra or frontend-dapp/.env.local not ready"
      skip "chain: Playwright wrap-swap E10"
    fi
  fi
fi

query_columbus_tx() {
  local hash="$1"
  local url="${COLUMBUS_LCD}/cosmos/tx/v1beta1/txs/${hash}"
  echo "  LCD GET ${url}"
  local body
  if ! body="$(curl -fsS --max-time 30 "${url}")"; then
    echo "columbus-5 LCD fetch failed for ${hash}" >&2
    return 1
  fi
  local wanted used code
  wanted="$(python3 -c 'import json,sys; d=json.loads(sys.argv[1]); print(d.get("tx_response",{}).get("gas_wanted",""))' "${body}")"
  used="$(python3 -c 'import json,sys; d=json.loads(sys.argv[1]); print(d.get("tx_response",{}).get("gas_used",""))' "${body}")"
  code="$(python3 -c 'import json,sys; d=json.loads(sys.argv[1]); print(d.get("tx_response",{}).get("code",""))' "${body}")"
  echo "  code=${code} gasWanted=${wanted} gasUsed=${used} hash=${hash}"
  python3 -c '
import sys
wanted, used, code = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
envelope = int(sys.argv[4])
if used <= 0 or wanted <= 0:
    raise SystemExit("invalid gas fields")
if code != 0:
    raise SystemExit(f"tx code={code} (not success)")
if used >= wanted:
    raise SystemExit(f"AC1 class A (OOG): gasUsed {used} >= gasWanted {wanted} — raise WRAP_ROUTER_COMBO_OVERHEAD_GAS in a follow-up, not here")
if wanted > 15_000_000:
    raise SystemExit(f"AC1 class B (wallet auto-gas): gasWanted {wanted} >> {envelope} wrap+2hop envelope")
if used >= envelope:
    raise SystemExit(f"gasUsed {used} >= {envelope} envelope — open AC2; do not bump WRAP_GAS_LIMIT")
print(f"  AC1 not-A: margin={wanted-used} (used < {envelope} floor ok; record wallet rewrite vs hint as B/C on the issue)")
' "${wanted}" "${used}" "${code}" "${WRAP_2HOP_ENVELOPE}"
}

if [[ -n "${VERIFY1264_COLUMBUS_TX:-}" ]]; then
  run_step "columbus-5 LCD gas_used < gas_wanted (AC1)" \
    query_columbus_tx "${VERIFY1264_COLUMBUS_TX}"
else
  if [[ "${VERIFY1264_REQUIRE_MAINNET:-0}" = "1" ]]; then
    echo ""
    echo "[columbus-5]"
    bad "columbus-5 hash required (VERIFY1264_REQUIRE_MAINNET=1) — set VERIFY1264_COLUMBUS_TX"
  else
    echo ""
    echo "[columbus-5] SKIP — set VERIFY1264_COLUMBUS_TX=<hash> after operator USTC→USTR"
    skip "columbus-5 USTC→USTR (AC1) — set VERIFY1264_COLUMBUS_TX=<hash>"
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
echo "==> Forgejo #1264 verification passed (envelope unchanged; AC1 measured not-A/not-B; optional VERIFY1264_COLUMBUS_TX LCD pin)"
