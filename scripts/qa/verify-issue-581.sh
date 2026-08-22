#!/usr/bin/env bash
# Verification for GitLab #581 — CW20 8266 SpaceUSD/Terraport template GO/NO-GO.
#
# Ops note (issue comment 3719458992): issuer wasm-admin, Everybody instantiate,
# and minter cap are documented residuals — they do not block listing. The remaining
# gate is the full Layer A-lcd + B-lt suite on the pinned LCD wasm.
#
# Requires LocalTerra for the execution rungs (SKIP unless VERIFY581_REQUIRE_CHAIN=1).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }
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
echo "  GitLab #581 — CW20 code 8266 full suite + listing residuals"
echo "════════════════════════════════════════════════════════════════"

AUDIT="$REPO_ROOT/cw20-codeid-audits"
REPORT="$AUDIT/codeids/8266/REPORT.md"

run_step "docs: 8266 pin + issuer/Everybody/minter not blocking" \
  bash -c "
    set -euo pipefail
    pin=\$(tr -d '[:space:]' < '$AUDIT/codeids/8266/wasm.sha256' | tr '[:lower:]' '[:upper:]')
    test \"\$pin\" = '953AD60CF6D8C9631B99ADC84C3ABF4083815743F86FF81B2A422FDFDF5F95C0'
    rg -qF '**GO**' '$REPORT'
    rg -q 'issuer' '$REPORT'
    rg -q 'Everybody' '$REPORT'
    rg -q 'minter' '$REPORT'
    rg -q 'not blocking' '$REPORT'
    rg -q '100 LUNC' docs/runbooks/cw20-whitelist-policy.md || rg -q 'pair-create fee' '$REPORT'
    rg -q 'verify-issue-581' Makefile
  "

if timeout 20 make -s has-localterra >/dev/null 2>&1; then
  run_step "CODE_ID=8266 LAYER_B_LT=1 make verify-issue-589 (full suite)" \
    bash -c 'CODE_ID=8266 LAYER_B_LT=1 make verify-issue-589'
  run_step "REPORT records executed Send/TransferFrom/round-trip/limit" \
    bash -c "
      set -euo pipefail
      rg -q 'TransferFrom' '$REPORT'
      rg -q 'round-trip' '$REPORT' || rg -q 'B7' '$REPORT'
      rg -q 'limit' '$REPORT'
    "
else
  run_step "child: make verify-issue-589 (A-mt/B-mt + docs; no chain)" make verify-issue-589
  if [[ "${VERIFY581_REQUIRE_CHAIN:-0}" == "1" ]]; then
    bad "LocalTerra required (VERIFY581_REQUIRE_CHAIN=1) — make setup-cloud-localterra"
  else
    skip "Layer A-lcd/B-lt (make has-localterra). Cloud Agent: make setup-cloud-localterra"
  fi
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #581 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
exit 0
