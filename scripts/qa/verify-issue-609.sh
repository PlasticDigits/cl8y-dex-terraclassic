#!/usr/bin/env bash
# Automated verification for GitLab #609 — ExemptionDirectory full tax skip (M-5).
#
# Proves (unit + docs, twice):
#   1. Token multitest: exempt sell/buy/transfer Honest; guards stay on
#   2. Frontend hint + extra-debit Max fail-closed
#   3. T592-7 / E609 + skill crosslinks
#
# Refs: skills/AGENTS_COMMUNITY_TAX_EXEMPT.md
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
echo "  GitLab #609 — ExemptionDirectory skips buy/sell/transfer tax"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
if [[ -e "$REPO_ROOT/smartcontracts/target" && ! -w "$REPO_ROOT/smartcontracts/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-609-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi

run_crates() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-token --offline -- --test-threads=1)
}

run_frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/communityTaxSku.test.ts \
    src/utils/taxPreviewMaxSpend.test.ts
}

run_docs() {
  set -euo pipefail
  rg -q "E609-1" skills/AGENTS_COMMUNITY_TAX_EXEMPT.md
  rg -q "AGENTS_COMMUNITY_TAX_EXEMPT" AGENTS.md
  rg -q "verify-issue-609" AGENTS.md
  rg -q "verify-issue-609" docs/testing.md
  rg -q "MANAGER_EXEMPT" docs/contracts-terraclassic.md
  rg -q "E609-1" docs/contracts-security-audit.md
  rg -q "classify_trade" smartcontracts/contracts/community-tax-token/src/tax.rs
  rg -q "is_manager_directory_tax_skip" smartcontracts/contracts/community-tax-token/src/tax.rs
  rg -q "skip buy, sell, and transfer tax" frontend-dapp/src/utils/communityTaxSku.ts
  rg -q "effectiveExtraDebitSellBps" frontend-dapp/src/utils/taxPreviewMaxSpend.ts
  rg -q "queryCommunityTaxIsExempt" frontend-dapp/src/hooks/useCommunityTaxSellBps.ts
  rg -q "T592-11" smartcontracts/contracts/community-tax-token/src/tax.rs
}

echo ""
echo "── first pass ──"
run_step "crates: cl8y-community-tax-token" run_crates
run_step "frontend: SKU hint + extra-debit Max" run_frontend
run_step "docs: E609 + T592-7 + skill + hint" run_docs

echo ""
echo "── retest ──"
run_step "retest crates: cl8y-community-tax-token" run_crates
run_step "retest frontend: SKU hint + extra-debit Max" run_frontend
run_step "retest docs: E609 + T592-7 + skill + hint" run_docs

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #609 verification passed"
