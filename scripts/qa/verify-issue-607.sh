#!/usr/bin/env bash
# Automated verification for GitLab #607 — community tax router hops Honest (C-2 option 1).
#
# Design close (disclose-only). Proves:
#   1. T592-13 / C593-14 / R607 written in skills + docs
#   2. Classify still skips protocol-exempt from/to (no option 2/3)
#   3. PoC stays as a documented property
#   4. dApp copy + extra-debit Max helpers
#
# Refs: skills/AGENTS_COMMUNITY_TAX_ROUTER.md
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
echo "  GitLab #607 — community tax router hops Honest (option 1)"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
if [[ -e "$REPO_ROOT/smartcontracts/target" && ! -w "$REPO_ROOT/smartcontracts/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-607-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi

run_docs() {
  set -euo pipefail
  rg -q "T592-13" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "C593-14" skills/AGENTS_FRONTEND_CREATE_TOKEN.md
  rg -q "R607-1" skills/AGENTS_COMMUNITY_TAX_ROUTER.md
  rg -q "option 1" skills/AGENTS_COMMUNITY_TAX_ROUTER.md
  rg -q "T592-13" docs/contracts-terraclassic.md
  rg -q "T592-13" docs/contracts-security-audit.md
  rg -q "C593-14" docs/frontend.md
  rg -q "verify-issue-607" docs/testing.md
  rg -q "AGENTS_COMMUNITY_TAX_ROUTER" AGENTS.md
  rg -q "T592-13" skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md
  rg -q "T592-13" smartcontracts/contracts/community-tax-token/src/tax.rs
  rg -q "T592-13 / #607 option 1" smartcontracts/contracts/community-token-launcher/tests/audit_poc.rs
  rg -q "pair-direct only" frontend-dapp/src/utils/taxPreviewMaxSpend.ts
  rg -q "Route skips buy/sell tax" frontend-dapp/src/utils/taxPreviewMaxSpend.ts
  rg -q "communityTaxRouteHint" frontend-dapp/src/pages/SwapPage.tsx
  rg -q "communityTaxRouteHint" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx
  rg -q "create-token-tax-scope" frontend-dapp/src/pages/CreateTokenPage.tsx
  rg -q "manage-token-tax-scope" frontend-dapp/src/pages/ManageTokenPage.tsx
  # Option 2/3 must not have been implemented: sell still requires !is_protocol_exempt(from).
  rg -q "!is_protocol_exempt\\(storage, self_addr, from\\)" smartcontracts/contracts/community-tax-token/src/tax.rs
  rg -q "!is_protocol_exempt\\(storage, self_addr, to\\)" smartcontracts/contracts/community-tax-token/src/tax.rs
}

run_frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/taxPreviewMaxSpend.test.ts \
    src/pages/CreateTokenPage.test.tsx \
    src/pages/ManageTokenPage.test.tsx
}

run_poc() {
  (cd smartcontracts && cargo test -p cl8y-community-token-launcher --offline --test audit_poc poc_router_exemption_full_tax_bypass -- --test-threads=1)
}

echo ""
echo "── first pass ──"
run_step "docs: T592-13 + C593-14 + R607 + classify guard" run_docs
run_step "frontend: route hint + Create/Manage copy" run_frontend
run_step "poc: router hops Honest (documented property)" run_poc

echo ""
echo "── retest ──"
run_step "retest docs: T592-13 + C593-14 + R607 + classify guard" run_docs
run_step "retest frontend: route hint + Create/Manage copy" run_frontend
run_step "retest poc: router hops Honest (documented property)" run_poc

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #607 verification passed"
