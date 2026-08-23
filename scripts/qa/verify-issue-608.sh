#!/usr/bin/env bash
# Automated verification for GitLab #608 — LaunchGuards cooldown / max_wallet liveness.
#
# Proves (unit + docs, twice):
#   1. Token launch-guard multitest (per-wallet cooldown, provide after cap)
#   2. Launcher audit_poc H-3 / H-4 inverted
#   3. H608 / T592-11 skill + invariant crosslinks
#
# Refs: skills/AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS.md
# Does not store/migrate columbus-5 11611 (ops / F6 pin).
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
echo "  GitLab #608 — LaunchGuards per-wallet cooldown + provide liveness"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
if [[ -e "$REPO_ROOT/smartcontracts/target" && ! -w "$REPO_ROOT/smartcontracts/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-608-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi

run_token() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-token --offline -- --test-threads=1)
}

run_pocs() {
  (cd smartcontracts && cargo test -p cl8y-community-token-launcher --test audit_poc --offline -- --test-threads=1)
}

run_docs() {
  set -euo pipefail
  rg -q "H608-1" docs/contracts-security-audit.md
  rg -q "H608-1" skills/AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS.md
  rg -q "AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "per user wallet" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "verify-issue-608" AGENTS.md
  rg -q "verify-issue-608" docs/testing.md
  rg -q "LaunchGuards (T592-11 / #608)" docs/contracts-terraclassic.md
  rg -q "is_cooldown_subject" smartcontracts/contracts/community-tax-token/src/tax.rs
  rg -q "H608-1" smartcontracts/contracts/community-tax-token/src/state.rs
  rg -q "inverted" smartcontracts/contracts/community-token-launcher/tests/audit_poc.rs
  rg -q "crate #608" cw20-codeid-audits/codeids/11611/REPORT.md
  rg -q "AGENTS_COMMUNITY_TAX_LAUNCH_GUARDS" cw20-codeid-audits/codeids/community-tax-token/REPORT.md
}

echo ""
echo "── first pass ──"
run_step "crates: community-tax-token" run_token
run_step "crates: audit_poc H-3/H-4 inverted" run_pocs
run_step "docs: H608 + T592-11 + 11611 D11" run_docs

echo ""
echo "── retest ──"
run_step "retest crates: community-tax-token" run_token
run_step "retest crates: audit_poc H-3/H-4 inverted" run_pocs
run_step "retest docs: H608 + T592-11 + 11611 D11" run_docs

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #608 verification passed"
