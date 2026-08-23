#!/usr/bin/env bash
# Automated verification for GitLab #605 — SKU init + retail percent taxes.
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
echo "  GitLab #605 — SKU initialization + percent tax inputs"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
if [[ -e "$REPO_ROOT/smartcontracts/target" && ! -w "$REPO_ROOT/smartcontracts/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-605-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi

run_crates() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-token -p cl8y-community-token-launcher -p cl8y-community-tax-autolp --offline -- --test-threads=1 instantiate_requires_explicit instantiate_launch_guards instantiate_rejects_transfer create_token_autolp create_token_instantiates)
}

run_frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/communityTaxSku.test.ts \
    src/utils/communityTaxInvoice.test.ts \
    src/utils/communityTaxCreateForm.test.ts \
    src/pages/CreateTokenPage.test.tsx \
    src/pages/ManageTokenPage.test.tsx
}

run_docs() {
  rg -q "C605-1" docs/frontend.md
  rg -q "parseTaxPercent" frontend-dapp/src/utils/communityTaxSku.ts
  rg -q "BindAutolp" smartcontracts/contracts/community-tax-token/src/msg.rs
  rg -q "initial_exempt" smartcontracts/contracts/community-tax-token/src/msg.rs
  rg -q "C605-1" skills/AGENTS_FRONTEND_CREATE_TOKEN.md
  rg -q "AutoLP instantiate+bind" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "verify-issue-605" docs/testing.md
  rg -q "verify-issue-605" AGENTS.md
  ! rg -q "Buy tax \\(bps\\)" frontend-dapp/src/pages/CreateTokenPage.tsx
  ! rg -q "Buy bps" frontend-dapp/src/pages/ManageTokenPage.tsx
}

echo ""
echo "── first pass ──"
run_step "crates: SKU init + AutoLP bind" run_crates
run_step "frontend: percent + SKU panels" run_frontend
run_step "docs: C605 + no retail bps labels" run_docs

echo ""
echo "── retest ──"
run_step "retest crates: SKU init + AutoLP bind" run_crates
run_step "retest frontend: percent + SKU panels" run_frontend
run_step "retest docs: C605 + no retail bps labels" run_docs

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #605 verification passed"
