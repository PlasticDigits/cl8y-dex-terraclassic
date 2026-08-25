#!/usr/bin/env bash
# Automated verification for GitLab #633 — listed-pair autoregister +
# Manage catch-up + manager role tax skip.
#
# Proves (unit + docs, twice):
#   1. Token: manager role skip without MANAGER_EXEMPT
#   2. AutoLP: set-pair / instantiate register-on-bind
#   3. Factory: cw2-gated tax register msgs
#   4. Frontend: Manage alert + highest-LP + Create Pair follow-up
#   5. R633 + skill crosslinks
#
# Refs: skills/AGENTS_COMMUNITY_TAX_AUTOREGISTER.md
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
echo "  GitLab #633 — autoregister + manager role tax skip"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
if [[ -e "$REPO_ROOT/smartcontracts/target" && ! -w "$REPO_ROOT/smartcontracts/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-633-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi

run_crates() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-token -p cl8y-community-tax-autolp -p cl8y-dex-factory --offline -- --test-threads=1)
}

run_frontend() {
  if [[ ! -x frontend-dapp/node_modules/.bin/vitest ]]; then
    bash scripts/with-node.sh --cwd frontend-dapp -- npm ci
  fi
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/communityTaxRegisterPair.test.ts \
    src/components/community/ManageUnregisteredPairAlert.test.tsx \
    src/pages/ManageTokenPage.test.tsx \
    src/pages/CreatePairPage.test.tsx
}

run_docs() {
  set -euo pipefail
  rg -q "R633-1" skills/AGENTS_COMMUNITY_TAX_AUTOREGISTER.md
  rg -q "AGENTS_COMMUNITY_TAX_AUTOREGISTER" AGENTS.md
  rg -q "verify-issue-633" AGENTS.md
  rg -q "verify-issue-633" docs/testing.md
  rg -q "R633-1" docs/contracts-terraclassic.md
  rg -q "R633-1" docs/contracts-security-audit.md
  rg -q "is_manager_exempt" smartcontracts/contracts/community-tax-token/src/tax.rs
  rg -q "COMMUNITY_TAX_CW2_NAME" smartcontracts/contracts/factory/src/tax_register.rs
  rg -q "register_listed_pair_msg" smartcontracts/contracts/community-tax-autolp/src/pair.rs
  rg -q "registerTaxAssetsAfterCreatePair" frontend-dapp/src/utils/communityTaxRegisterPair.ts
  rg -q "manage-register-alert" frontend-dapp/src/components/community/ManageUnregisteredPairAlert.tsx
  rg -q "This market is not collecting buy/sell tax yet" frontend-dapp/src/utils/communityTaxRegisterPair.ts
}

echo ""
echo "── first pass ──"
run_step "crates: token + autolp + factory" run_crates
run_step "frontend: register picker + Manage + Create Pair" run_frontend
run_step "docs: R633 + skill + copy" run_docs

echo ""
echo "── retest ──"
run_step "retest crates: token + autolp + factory" run_crates
run_step "retest frontend: register picker + Manage + Create Pair" run_frontend
run_step "retest docs: R633 + skill + copy" run_docs

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #633 verification passed"
