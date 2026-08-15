#!/usr/bin/env bash
# Verification for GitLab #518 — LP ticker keeps 0-9; factory code-id upgrade.
#
# Refs: docs/contracts-terraclassic.md#createpair-lp-ticker-gitlab-518
#       docs/contracts-security-audit.md (F3)
#       skills/AGENTS_LP_SYMBOL_DIGITS.md
#       scripts/upgrade-518-lp-symbol.sh
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
echo "  GitLab #518 — LP ticker digits + factory upgrade"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_dex_common() {
  (cd smartcontracts && cargo test -p dex-common lp_symbol -- --quiet)
}

run_integration() {
  (cd smartcontracts && cargo test -p cl8y-dex-tests --lib lp_symbol -- --quiet)
}

run_docs() {
  set -euo pipefail
  rg -q "createpair-lp-ticker-gitlab-518" docs/contracts-terraclassic.md
  rg -q "F3" docs/contracts-security-audit.md
  rg -q "is_mintable_cw20_lp_symbol" docs/contracts-security-audit.md
  rg -q "UST1-CUST-LP" docs/contracts-security-audit.md
  rg -q "pair_code_id" docs/contracts-terraclassic.md
  rg -q "AGENTS_LP_SYMBOL_DIGITS" AGENTS.md
  rg -q "upgrade-518-lp-symbol" skills/AGENTS_LP_SYMBOL_DIGITS.md
  rg -q "518" skills/AGENTS_UST1_SECONDARY_AMM.md
  test -f skills/AGENTS_LP_SYMBOL_DIGITS.md
  test -f scripts/upgrade-518-lp-symbol.sh
  rg -q "lp_token_instantiate_meta" smartcontracts/contracts/pair/src/contract.rs
  rg -q "lp_token_code_id" smartcontracts/packages/dex-common/src/factory.rs
}

run_upgrade_script() {
  set -euo pipefail
  chmod +x scripts/upgrade-518-lp-symbol.sh
  bash -n scripts/upgrade-518-lp-symbol.sh
  DRY_RUN=1 UPGRADE518_SKIP_STORE=1 UPGRADE518_SKIP_FACTORY_MIGRATE=1 \
    UPGRADE518_PAIR_CODE_ID=10 UPGRADE518_FACTORY_CODE_ID=11 UPGRADE518_LP_CODE_ID=12 \
    UPGRADE518_FACTORY_ADDRESS=terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea \
    ./scripts/upgrade-518-lp-symbol.sh
}

echo ""
echo "── first pass ──"
run_step "dex-common: lp_symbol unit tests" run_dex_common
run_step "integration: classic revert + UpdateConfig unblock" run_integration
run_step "docs: F3 + skill + upgrade script" run_docs
run_step "upgrade script: bash -n + DRY_RUN" run_upgrade_script

echo ""
echo "── retest ──"
run_step "retest dex-common: lp_symbol unit tests" run_dex_common
run_step "retest integration: classic revert + UpdateConfig unblock" run_integration
run_step "retest upgrade script: DRY_RUN" run_upgrade_script

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #518 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "OK"
