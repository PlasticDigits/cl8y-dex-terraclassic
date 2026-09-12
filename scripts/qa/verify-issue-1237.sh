#!/usr/bin/env bash
# Automated verification for Forgejo #1237 — SettingsBatch AutoLP / minter no-op (T592-4).
#
# Proves (unit + docs, twice):
#   1. Token crate: identical autolp / mixed batch / minter / launch_guards no-op
#   2. AutoLP crate merge / pair gate unchanged
#   3. Manage Token + invoice helper: equal AutoLP does not attach
#   4. T592-4 sister-identity docs / skills
#
# Refs: skills/AGENTS_COMMUNITY_TAX_CW20.md T592-4
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
echo "  Forgejo #1237 — AutoLP / minter settings no-op (T592-4)"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
if [[ -e "$REPO_ROOT/smartcontracts/target" && ! -w "$REPO_ROOT/smartcontracts/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-1237-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi

run_token() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-token --offline -- --test-threads=1 \
    identical_autolp_is_noop_no_cmm_credit \
    identical_autolp_with_buy_bps_delta_invoices_without_sister_write \
    autolp_threshold_delta_invoices_and_updates_sister \
    omitted_autolp_pair_does_not_clear_and_is_noop_when_equal \
    unbound_autolp_settings_reverts_fee_not_kept \
    autolp_sku_off_reverts_fee_not_kept \
    autolp_fake_pair_reverts_fee_not_kept \
    autolp_new_listed_pair_invoices_and_sets_sister \
    autolp_skim_delta_invoices \
    identical_minter_is_noop \
    identical_launch_guards_is_noop \
    settings_empty_batch_is_noop \
    settings_batch_flat_fee_and_noop)
}

run_autolp() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-autolp --offline -- --test-threads=1)
}

run_frontend() {
  if [[ ! -x frontend-dapp/node_modules/.bin/vitest ]]; then
    bash scripts/with-node.sh --cwd frontend-dapp -- npm ci
  fi
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/communityTaxInvoice.test.ts \
    src/pages/ManageTokenPage.test.tsx
}

run_docs() {
  set -euo pipefail
  rg -q "T592-4" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "sister \`GetConfig\` identity" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "verify-issue-1237" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "verify-issue-1237" skills/AGENTS_COMMUNITY_TAX_AUTOLP.md
  rg -q "T592-4" skills/AGENTS_COMMUNITY_TAX_AUTOLP.md
  rg -q "buildAutolpSettingsDelta" skills/AGENTS_FRONTEND_CREATE_TOKEN.md \
    || rg -q "GetConfig differs" skills/AGENTS_FRONTEND_CREATE_TOKEN.md
  rg -q "verify-issue-1237" AGENTS.md
  rg -q "verify-issue-1237" docs/testing.md
  rg -q "sister \`GetConfig\` identity" docs/contracts-terraclassic.md
  rg -q "query_wasm_smart" smartcontracts/contracts/community-tax-token/src/invoice.rs
  rg -q "identical_autolp_is_noop" smartcontracts/contracts/community-tax-token/src/multitest.rs
  rg -q "buildAutolpSettingsDelta" frontend-dapp/src/utils/communityTaxInvoice.ts
  rg -q "queryAutoLpConfig" frontend-dapp/src/services/terraclassic/communityTaxToken.ts
}

echo ""
echo "── first pass ──"
run_step "crates: identical-autolp / mixed-batch / minter no-op" run_token
run_step "crates: cl8y-community-tax-autolp" run_autolp
run_step "frontend: invoice delta + Manage Token RTL" run_frontend
run_step "docs: T592-4 AutoLP sister identity" run_docs

echo ""
echo "── retest ──"
run_step "retest crates: identical-autolp / mixed-batch / minter no-op" run_token
run_step "retest crates: cl8y-community-tax-autolp" run_autolp
run_step "retest frontend: invoice delta + Manage Token RTL" run_frontend
run_step "retest docs: T592-4 AutoLP sister identity" run_docs

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> Forgejo #1237 verification passed"
