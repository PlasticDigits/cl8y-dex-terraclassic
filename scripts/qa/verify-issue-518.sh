#!/usr/bin/env bash
# Verification for GitLab #518 — LP ticker classic charset (UST1 / CL8Y create_pair).
#
# Contract unit + multi-tests only (no chain). Retest is the same suite run twice.
#
# Refs: docs/contracts-terraclassic.md#createpair-lp-ticker-gitlab-518
#       docs/contracts-security-audit.md (F3)
#       skills/AGENTS_LP_SYMBOL_DIGITS.md
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
echo "  GitLab #518 — LP ticker digits / classic charset"
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
  rg -q "is_classic_cw20_lp_symbol" docs/contracts-security-audit.md
  rg -q "CLY-LP" docs/contracts-security-audit.md
  rg -q "AGENTS_LP_SYMBOL_DIGITS" AGENTS.md
  rg -q "518" skills/AGENTS_UST1_SECONDARY_AMM.md
  test -f skills/AGENTS_LP_SYMBOL_DIGITS.md
  rg -q "FALLBACK_LP_TOKEN_SYMBOL" smartcontracts/packages/dex-common/src/lp_symbol.rs
  rg -q "lp_token_instantiate_meta" smartcontracts/contracts/pair/src/contract.rs
}

echo ""
echo "── first pass ──"
run_step "dex-common: lp_symbol unit tests" run_dex_common
run_step "integration: create_pair UST1/CL8Y on classic LP CW20" run_integration
run_step "docs: F3 + skill + UST1 crosslink" run_docs

echo ""
echo "── retest ──"
run_step "retest dex-common: lp_symbol unit tests" run_dex_common
run_step "retest integration: create_pair UST1/CL8Y on classic LP CW20" run_integration

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
