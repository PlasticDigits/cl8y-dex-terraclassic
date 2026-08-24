#!/usr/bin/env bash
# Automated verification for GitLab #610 — AutoLP factory-listed pair + skim floor (M-2 / M-3).
#
# Proves (unit + docs, twice):
#   1. AutoLP crate: factory pair, floor, merge, permissionless skim
#   2. Inverted audit PoC poc_autolp_manager_can_skim_to_fake_pair
#   3. Launcher stamps immutable factory
#   4. T592-10 / M610 + Manage Token / SKU copy
#
# Refs: skills/AGENTS_COMMUNITY_TAX_AUTOLP.md
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
echo "  GitLab #610 — AutoLP factory pair + skim floor (M-2 / M-3)"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
if [[ -e "$REPO_ROOT/smartcontracts/target" && ! -w "$REPO_ROOT/smartcontracts/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-610-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi

run_autolp() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-autolp --offline -- --test-threads=1)
}

run_poc() {
  (cd smartcontracts && cargo test -p cl8y-community-token-launcher --offline --lib \
    create_token_instantiates_and_binds_autolp -- --test-threads=1)
  (cd smartcontracts && cargo test -p cl8y-community-token-launcher --offline --test audit_poc \
    poc_autolp_manager_can_skim_to_fake_pair -- --test-threads=1)
}

run_frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/communityTaxSku.test.ts \
    src/pages/ManageTokenPage.test.tsx
}

run_docs() {
  set -euo pipefail
  rg -q "M610-1" skills/AGENTS_COMMUNITY_TAX_AUTOLP.md
  rg -q "AGENTS_COMMUNITY_TAX_AUTOLP" AGENTS.md
  rg -q "verify-issue-610" AGENTS.md
  rg -q "verify-issue-610" docs/testing.md
  rg -q "M610-1" docs/contracts-security-audit.md
  rg -q "factory-listed" docs/contracts-terraclassic.md
  rg -q "T592-10" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "factory-listed with this token" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "require_factory_listed_tax_pair" smartcontracts/contracts/community-tax-autolp/src/pair.rs
  rg -q "max_spread: Some" smartcontracts/contracts/community-tax-autolp/src/contract.rs
  rg -q "this token’s CL8Y factory pair" frontend-dapp/src/pages/ManageTokenPage.tsx \
    || rg -q "this token's CL8Y factory pair" frontend-dapp/src/pages/ManageTokenPage.tsx
  rg -q "factory-listed" frontend-dapp/src/utils/communityTaxSku.ts
}

echo ""
echo "── first pass ──"
run_step "crates: cl8y-community-tax-autolp" run_autolp
run_step "crates: inverted M-3 PoC + launcher factory pin" run_poc
run_step "frontend: SKU hint + Manage Token pair copy" run_frontend
run_step "docs: M610 + T592-10 + skill + copy" run_docs

echo ""
echo "── retest ──"
run_step "retest crates: cl8y-community-tax-autolp" run_autolp
run_step "retest crates: inverted M-3 PoC + launcher factory pin" run_poc
run_step "retest frontend: SKU hint + Manage Token pair copy" run_frontend
run_step "retest docs: M610 + T592-10 + skill + copy" run_docs

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #610 verification passed"
