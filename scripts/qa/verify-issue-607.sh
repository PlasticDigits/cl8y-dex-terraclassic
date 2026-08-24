#!/usr/bin/env bash
# Automated verification for GitLab #607 — community tax router hops tax the
# original trader (C-2 improved option 2).
#
# Proves:
#   1. T592-13 / C593-14 / R607 written in skills + docs (option 2)
#   2. Classify taxes official-router Send+Swap; fail-closes without trader
#   3. Inverted PoC + token multitest
#   4. dApp extra-debit Max + listed-pair copy
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
echo "  GitLab #607 — community tax router hop tax (improved option 2)"
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
  rg -q "improved option 2" skills/AGENTS_COMMUNITY_TAX_ROUTER.md
  rg -q "T592-13" docs/contracts-terraclassic.md
  rg -q "T592-13" docs/contracts-security-audit.md
  rg -q "C593-14" docs/frontend.md
  rg -q "verify-issue-607" docs/testing.md
  rg -q "AGENTS_COMMUNITY_TAX_ROUTER" AGENTS.md
  rg -q "T592-13" skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md
  rg -q "T592-13" smartcontracts/contracts/community-tax-token/src/tax.rs
  rg -q "T592-13 / #607 improved option 2" smartcontracts/contracts/community-token-launcher/tests/audit_poc.rs
  rg -q "Buy/sell tax applies on every listed-pair swap" frontend-dapp/src/utils/taxPreviewMaxSpend.ts
  rg -q "Sell tax extra" frontend-dapp/src/utils/taxPreviewMaxSpend.ts
  rg -q "Buy tax applies" frontend-dapp/src/utils/taxPreviewMaxSpend.ts
  rg -q "communityTaxRouteHint" frontend-dapp/src/pages/SwapPage.tsx
  rg -q "communityTaxRouteHint" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx
  rg -q "create-token-tax-scope" frontend-dapp/src/pages/CreateTokenPage.tsx
  rg -q "manage-token-tax-scope" frontend-dapp/src/pages/ManageTokenPage.tsx
  rg -q "RouterTraderRequired" smartcontracts/contracts/community-tax-token/src/error.rs
  rg -q "is_official_router" smartcontracts/contracts/community-tax-token/src/tax.rs
  rg -q "hop_trader_addr" smartcontracts/contracts/community-tax-token/src/tax.rs
  # Option 1 disclose-only copy must not remain as current policy.
  if rg -q "Route skips buy/sell tax" frontend-dapp/src/utils/taxPreviewMaxSpend.ts; then
    echo "stale option 1 skip hint" >&2
    return 1
  fi
  if rg -q "Buy/sell tax is pair-direct only" frontend-dapp/src/utils/taxPreviewMaxSpend.ts; then
    echo "stale option 1 Create copy" >&2
    return 1
  fi
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

run_token() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-token --offline --lib -- --test-threads=1 \
    router_sell_extra_debits_authenticated_trader \
    router_sell_without_trader_fail_closes \
    router_sell_rejects_protocol_exempt_trader \
    pair_direct_ignores_spoofed_trader \
    router_to_user_is_buy_outbound_split \
    manager_exempt_hop_trader_skips_router_sell_tax)
}

echo ""
echo "── first pass ──"
run_step "docs: T592-13 + C593-14 + R607 option 2" run_docs
run_step "frontend: extra-debit Max + Create/Manage copy" run_frontend
run_step "token: router hop classify + extra-debit" run_token
run_step "poc: inverted router hop tax" run_poc

echo ""
echo "── retest ──"
run_step "retest docs: T592-13 + C593-14 + R607 option 2" run_docs
run_step "retest frontend: extra-debit Max + Create/Manage copy" run_frontend
run_step "retest token: router hop classify + extra-debit" run_token
run_step "retest poc: inverted router hop tax" run_poc

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
