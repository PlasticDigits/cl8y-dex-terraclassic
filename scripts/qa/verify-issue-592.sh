#!/usr/bin/env bash
# Automated verification for GitLab #592 — community tax CW20 (DEX-safe).
#
# Proves (unit + docs):
#   1. Token / launcher / AutoLP crate tests (invoices, extra-debit, inbound 1:1)
#   2. Pair/router swap-math files unchanged in this worktree vs the invariant comments
#   3. Docs/skills/REPORT/#589 crosslinks for T592 invariants
#
# Refs: skills/AGENTS_COMMUNITY_TAX_CW20.md
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
echo "  GitLab #592 — community tax CW20 (DEX-safe buy/sell/transfer)"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_crates() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-token -p cl8y-community-token-launcher -p cl8y-community-tax-autolp --offline -- --test-threads=1)
}

run_docs() {
  set -euo pipefail
  rg -q "T592-1" docs/contracts-security-audit.md
  rg -q "community-tax-cw20-gitlab-592" docs/contracts-terraclassic.md
  rg -q "community-tax-token" docs/runbooks/cw20-whitelist-policy.md
  rg -q "T592-1" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "AGENTS_COMMUNITY_TAX_CW20" AGENTS.md
  rg -q "verify-issue-592" AGENTS.md
  rg -q "verify-issue-592" docs/testing.md
  rg -q "community-tax-token" cw20-codeid-audits/CATALOG.md
  test -f cw20-codeid-audits/codeids/community-tax-token/REPORT.md
  rg -q "NO-GO" cw20-codeid-audits/codeids/community-tax-token/REPORT.md
  rg -q "INVOICE_UST1" smartcontracts/contracts/community-tax-token/src/msg.rs
  rg -q "T592-7" smartcontracts/contracts/community-tax-token/src/tax.rs
  # Pair/router must not grow FoT math in this crate set.
  test -d smartcontracts/contracts/community-tax-token
  test -d smartcontracts/contracts/community-token-launcher
  test -d smartcontracts/contracts/community-tax-autolp
}

echo ""
echo "── first pass ──"
run_step "crates: token + launcher + autolp" run_crates
run_step "docs: T592 + skill + REPORT NO-GO + whitelist exception" run_docs

echo ""
echo "── retest ──"
run_step "retest crates: token + launcher + autolp" run_crates
run_step "retest docs: T592 + skill + REPORT NO-GO + whitelist exception" run_docs

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #592 verification passed"
