#!/usr/bin/env bash
# Automated verification for GitLab #604 — Create Token identity + wallet helpers.
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
echo "  GitLab #604 — Create Token identity + connected-wallet helpers"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
if [[ -e "$REPO_ROOT/smartcontracts/target" && ! -w "$REPO_ROOT/smartcontracts/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-604-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi

run_crates() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-token -p cl8y-community-token-launcher --offline -- --test-threads=1 identity instantiate_rejects_decimals instantiate_accepts_decimals instantiate_rejects_bad_name create_token_rejects_decimals)
}

run_frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/communityTaxIdentity.test.ts \
    src/utils/communityTaxCreateForm.test.ts \
    src/pages/CreateTokenPage.test.tsx
}

run_docs() {
  rg -q "C604-1" docs/frontend.md
  rg -q "validate_identity" docs/contracts-terraclassic.md
  rg -q "C604-1" skills/AGENTS_FRONTEND_CREATE_TOKEN.md
  rg -q "11611" skills/AGENTS_FRONTEND_CREATE_TOKEN.md
  rg -q "MIN_DECIMALS" smartcontracts/contracts/community-tax-token/src/msg.rs
  rg -q "validate_identity" smartcontracts/contracts/community-tax-token/src/identity.rs
  rg -q "verify-issue-604" docs/testing.md
  rg -q "verify-issue-604" AGENTS.md
  rg -q "token/migrate" frontend-dapp/src/pages/CreateTokenPage.tsx
}

echo ""
echo "── first pass ──"
run_step "crates: identity instantiate" run_crates
run_step "frontend: identity + create page" run_frontend
run_step "docs: C604 + 11611 gap + Create Token links /token/migrate (#626)" run_docs

echo ""
echo "── retest ──"
run_step "retest crates: identity instantiate" run_crates
run_step "retest frontend: identity + create page" run_frontend
run_step "retest docs: C604 + 11611 gap + Create Token links /token/migrate (#626)" run_docs

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #604 verification passed"
