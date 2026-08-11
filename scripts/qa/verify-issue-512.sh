#!/usr/bin/env bash
# Automated verification for GitLab #512 — unwrap burn tax quotes + wrap display fix.
#
# Proves (unit + docs; no chain required):
#   1. Classic burn tax = floor(amount × rate); 9800×1.5% → 147 → 9653 receive.
#   2. Direct wrap quote = mapper fee only (9800 @ 200 bps), not tax+fee.
#   3. Direct unwrap quote = fee then tax; routerMinReceiveBase stays post-fee.
#   4. Fee note + exchange-deposit warning copy present.
#   5. Skills/docs/invariants W8–W11 crosslinked.
#
# Refs: skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md,
#       frontend-dapp/src/utils/nativeTransferTax.ts,
#       frontend-dapp/src/services/terraclassic/router.ts
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
echo "  GitLab #512 — unwrap burn tax + wrap mint quotes"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: nativeTransferTax + router + wrapMapper + pool + WrapPage" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/nativeTransferTax.test.ts \
    src/services/terraclassic/router.test.ts \
    src/services/terraclassic/__tests__/wrapMapper.test.ts \
    src/utils/__tests__/poolProvideCounterpart.test.ts \
    src/pages/WrapPage.test.tsx'

run_step "skill: AGENTS_WRAP_UNWRAP_BURN_TAX W8–W11" \
  grep -qE '\*\*W8\*\*' skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md && \
  grep -qE '\*\*W9\*\*' skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md && \
  grep -qE '\*\*W10\*\*' skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md && \
  grep -qE '\*\*W11\*\*' skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md

run_step "skill: AGENTS_MAINNET_WRAP_ENABLEMENT W8–W11" \
  grep -qE '\*\*W8\*\*' skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md && \
  grep -qE '\*\*W11\*\*' skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md

run_step "skill: AGENTS_NATIVE_WRAP_TAX #512 correction" \
  grep -qE '#512 correction' skills/AGENTS_NATIVE_WRAP_TAX.md

run_step "skill: AGENTS_ROUTER_MINIMUM_RECEIVE routerMinReceiveBase" \
  grep -qE 'routerMinReceiveBase' skills/AGENTS_ROUTER_MINIMUM_RECEIVE.md

run_step "docs: NATIVE_TOKEN_WRAPPING #512 tax table" \
  grep -qE 'InstantWithdraw.*Burn tax|#512' NATIVE_TOKEN_WRAPPING.md

run_step "docs: QA wrap-unwrap #512 checklist" \
  grep -qE '#512' docs/qa-templates/wrap-unwrap-test-pass.md

run_step "AGENTS.md playbook link #512" \
  grep -qE 'AGENTS_WRAP_UNWRAP_BURN_TAX|#512' AGENTS.md

run_step "code: netNativeAfterUnwrap helper" \
  grep -qE 'export async function netNativeAfterUnwrap' \
    frontend-dapp/src/services/terraclassic/router.ts

run_step "code: exchange-deposit warning copy" \
  grep -qE 'WRAP_UNWRAP_EXCHANGE_DEPOSIT_WARNING' \
    frontend-dapp/src/utils/marketDataServiceCopy.ts

run_step "code: burn_tax_rate LCD params path" \
  grep -qE 'terra/tax/v1beta1/params' \
    frontend-dapp/src/utils/nativeTransferTax.ts

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
exit 0
