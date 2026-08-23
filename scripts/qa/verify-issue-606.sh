#!/usr/bin/env bash
# Automated verification for GitLab #606 — launcher Enable Feature + SKU dedupe.
#
# Invariants T606-1–T606-8: skills/AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md
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
echo "  GitLab #606 — launcher Enable Feature + unique SKUs"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
if [[ -e "$REPO_ROOT/smartcontracts/target" && ! -w "$REPO_ROOT/smartcontracts/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-606-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi

run_crates() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-token -p cl8y-community-token-launcher --offline -- --test-threads=1)
}

run_pocs() {
  (cd smartcontracts && cargo test -p cl8y-community-token-launcher --test audit_poc \
    --offline -- poc_launcher --test-threads=1)
}

run_frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/communityTaxInvoice.test.ts
}

run_docs() {
  set -euo pipefail
  rg -q "T606-1" skills/AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md
  rg -q "T606-1" docs/contracts-security-audit.md
  rg -q "T606-1" docs/contracts-terraclassic.md
  rg -q "verify-issue-606" docs/testing.md
  rg -q "verify-issue-606" AGENTS.md
  rg -q "AGENTS_COMMUNITY_TAX_ENABLE_FEATURE" AGENTS.md
  rg -q "T606-1" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "T606-1" skills/AGENTS_FRONTEND_CREATE_TOKEN.md
  rg -qF 'Enable Feature 50 → **launcher**' docs/frontend.md
  rg -qF 'send_cw20_hook "$UST1_ADDR" "$LAUNCHER_ADDR"' scripts/qa/localterra-community-tax-smoke.sh
  rg -q "sku_unlock_via_launcher" scripts/qa/localterra-community-tax-smoke.sh
  rg -q "sku_unlock_via_launcher" scripts/qa/verify-issue-601.sh
  rg -q "assert_invoice_payer" smartcontracts/contracts/community-tax-token/src/invoice.rs
  rg -q "assert_unique_skus" smartcontracts/contracts/community-token-launcher/src/contract.rs
  # Official dApp path must stay launcher (not the no-migrate token workaround).
  rg -qF 'payee: input.launcher' frontend-dapp/src/utils/communityTaxInvoice.ts
  rg -qF 'enable_feature: { token: input.token' frontend-dapp/src/utils/communityTaxInvoice.ts
}

echo ""
echo "── first pass ──"
run_step "crates: token + launcher (incl. enable_feature + unique SKUs)" run_crates
run_step "PoCs: C-1 / H-2 inverted" run_pocs
run_step "frontend: Enable Feature payee + unique SKUs" run_frontend
run_step "docs: T606 + skill + smoke uses launcher" run_docs

echo ""
echo "── retest ──"
run_step "retest crates: token + launcher" run_crates
run_step "retest PoCs: C-1 / H-2 inverted" run_pocs
run_step "retest frontend: invoices" run_frontend
run_step "retest docs: T606 + skill + smoke uses launcher" run_docs

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #606 verification passed"
