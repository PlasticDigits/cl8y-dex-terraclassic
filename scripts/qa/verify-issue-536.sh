#!/usr/bin/env bash
# Verification for GitLab #536 — factory snapshots discount_registry on CreatePair.
#
# Refs: docs/contracts-terraclassic.md#factory-discount-registry-snapshot-gitlab-536
#       docs/contracts-security-audit.md (F5)
#       docs/reference/fee-discount-tiers.md (I14)
#       skills/AGENTS_FACTORY_DISCOUNT_REGISTRY.md
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
echo "  GitLab #536 — factory discount-registry snapshot"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_serde() {
  (cd smartcontracts && cargo test -p cl8y-dex-factory config_missing_discount_registry -- --quiet)
  (cd smartcontracts && cargo test -p dex-common pair_instantiate_omitted_discount_registry -- --quiet)
}

run_inherit() {
  (cd smartcontracts && cargo test -p cl8y-dex-tests --lib discount_registry_inherit -- --quiet)
}

run_docs() {
  set -euo pipefail
  rg -q "factory-discount-registry-snapshot-gitlab-536" docs/contracts-terraclassic.md
  rg -q "F5" docs/contracts-security-audit.md
  rg -q "I14" docs/reference/fee-discount-tiers.md
  rg -q "GetDiscountRegistry" smartcontracts/packages/dex-common/src/pair.rs
  rg -q "persist_factory_discount_registry" smartcontracts/contracts/factory/src/contract.rs
  rg -q "AGENTS_FACTORY_DISCOUNT_REGISTRY" AGENTS.md
  rg -q "536" skills/AGENTS_FEE_DISCOUNT_TIERS.md
  rg -q "536" skills/AGENTS_UST1_SECONDARY_AMM.md
  test -f skills/AGENTS_FACTORY_DISCOUNT_REGISTRY.md
  rg -q 'const CONTRACT_VERSION: &str = "1.8.0"' smartcontracts/contracts/factory/src/contract.rs
  rg -q 'const CONTRACT_VERSION: &str = "1.14.0"' smartcontracts/contracts/pair/src/contract.rs
  rg -q "getPairDiscountRegistry" frontend-dapp/src/services/terraclassic/settings.ts
  rg -q "set_discount_registry_all" scripts/deploy-dex-local.sh
}

echo ""
echo "── first pass ──"
run_step "serde: factory Config + PairInstantiateMsg default None" run_serde
run_step "integration: CreatePair inherit / single-pair isolation" run_inherit
run_step "docs: F5 + I14 + skill + versions" run_docs

echo ""
echo "── retest ──"
run_step "retest serde: factory Config + PairInstantiateMsg default None" run_serde
run_step "retest integration: CreatePair inherit / single-pair isolation" run_inherit

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #536 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "OK"
