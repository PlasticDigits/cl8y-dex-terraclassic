#!/usr/bin/env bash
# Verification for GitLab #582 — listed CW20 code_id pin + whitelist re-check.
#
# Refs: docs/contracts-terraclassic.md#asset-cw20-code-id-pin-gitlab-582
#       docs/contracts-security-audit.md (F6)
#       docs/runbooks/cw20-whitelist-policy.md
#       skills/AGENTS_CW20_CODE_ID_PIN.md
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
echo "  GitLab #582 — asset CW20 code_id pin + whitelist re-check"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_pin_tests() {
  (cd smartcontracts && cargo test -p cl8y-dex-tests --lib asset_code_id_pin -- --quiet)
}

run_docs() {
  set -euo pipefail
  rg -q "asset-cw20-code-id-pin-gitlab-582" docs/contracts-terraclassic.md
  rg -q "F6" docs/contracts-security-audit.md
  rg -q "IsCodeIdWhitelisted" smartcontracts/packages/dex-common/src/factory.rs
  rg -q "GetAssetCodeIds" smartcontracts/packages/dex-common/src/pair.rs
  rg -q "assert_asset_code_ids" smartcontracts/contracts/pair/src/asset_code_id_guard.rs
  rg -q "RefreshPairAssetCodeIds" smartcontracts/contracts/factory/src/contract.rs
  rg -q "AGENTS_CW20_CODE_ID_PIN" AGENTS.md
  test -f skills/AGENTS_CW20_CODE_ID_PIN.md
  rg -q 'const CONTRACT_VERSION: &str = "1.9.0"' smartcontracts/contracts/factory/src/contract.rs
  rg -q 'const CONTRACT_VERSION: &str = "1.15.0"' smartcontracts/contracts/pair/src/contract.rs
  rg -q "Listed-asset wasm admin inventory" docs/runbooks/cw20-whitelist-policy.md
  rg -q "582" docs/security-model.md
  rg -q "F6" docs/exploit-replay-matrix.md
}

echo ""
echo "── first pass ──"
run_step "integration: pin + FoT migrate + whitelist freeze + refresh" run_pin_tests
run_step "docs: F6 + skill + versions + cross-links" run_docs

echo ""
echo "── retest ──"
run_step "retest integration: pin + FoT migrate + whitelist freeze + refresh" run_pin_tests

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #582 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
