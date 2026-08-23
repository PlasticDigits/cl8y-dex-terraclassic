#!/usr/bin/env bash
# Automated verification for GitLab #595 — pay-with-any-token invoice module.
#
# Proves (unit + docs):
#   1. quotePayInvoice / buildPayInvoiceMsgs attack + quote tests
#   2. PayWithAnyToken card (disconnect gate, Pay CTA, No route, payee not from URL)
#   3. Gas inventory includes invoice Send + wrap+2hop+invoice combo
#   4. Docs/skills/AGENTS.md crosslinks for I595 invariants
#
# Refs: skills/AGENTS_FRONTEND_PAY_INVOICE.md,
#       frontend-dapp/src/utils/payInvoice.ts,
#       frontend-dapp/src/components/payments/PayWithAnyToken.tsx
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
echo "  GitLab #595 — pay with any token (DEX-routed invoice)"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: payInvoice quote + msgs + card + ceiling math" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/payInvoice.test.ts \
    src/utils/payInvoiceMsgs.test.ts \
    src/components/payments/PayWithAnyToken.test.tsx \
    src/utils/__tests__/rawAmountMath.test.ts'

run_step "frontend: retail gas inventory includes #595 combo" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/services/terraclassic/__tests__/terraGas.retailShapes.test.ts'

run_step "code: shared module + card exist for #593 consumers" \
  bash -c 'test -f frontend-dapp/src/utils/payInvoice.ts && \
    test -f frontend-dapp/src/components/payments/PayWithAnyToken.tsx'

run_step "code: PayWithAnyToken does not read payee from URL" \
  bash -c '! grep -qE "useSearchParams|URLSearchParams|location.search" \
    frontend-dapp/src/components/payments/PayWithAnyToken.tsx \
    frontend-dapp/src/utils/payInvoice.ts'

run_step "code: invoice Send gas constant + hook keys" \
  grep -qE 'PAY_INVOICE_SEND_GAS_LIMIT' frontend-dapp/src/utils/constants.ts

run_step "code: combined envelope wrap+2hop+invoice" \
  grep -qE 'wrap_plus_2hop_plus_invoice_send' frontend-dapp/src/services/terraclassic/terraGasRetailInventory.ts

run_step "docs: frontend.md #595 section + I595" \
  grep -qE 'I595-1' docs/frontend.md

run_step "docs: design-system glossary pay with any token" \
  grep -qE 'Pay with any token' docs/design-system.md

run_step "skill: AGENTS_FRONTEND_PAY_INVOICE I595 invariants" \
  grep -qE 'I595-1' skills/AGENTS_FRONTEND_PAY_INVOICE.md

run_step "skill: AGENTS.md playbook + verify-issue-595" \
  bash -c 'grep -qE "AGENTS_FRONTEND_PAY_INVOICE" AGENTS.md && grep -qE "verify-issue-595" AGENTS.md'

run_step "skill: hybrid quoting crosslink #595" \
  grep -qE '#595' skills/AGENTS_HYBRID_QUOTING.md

run_step "skill: gas playbook #595" \
  grep -qE '#595' skills/AGENTS_TERRACLASSIC_GAS.md

run_step "docs: testing.md verify-issue-595" \
  grep -qE 'verify-issue-595' docs/testing.md

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #595 verification passed"
