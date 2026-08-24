#!/usr/bin/env bash
# Verification for GitLab #623 — named tax-on Layer B suite
# (keep generic B-lt tax-off).
#
# Invariants C623-1–C623-8: skills/AGENTS_CW20_CODE_ID_TAX_ON.md
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
echo "  GitLab #623 — named tax-on Layer B (B-lt stays tax-off)"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
if [[ -e "$REPO_ROOT/smartcontracts/target" && ! -w "$REPO_ROOT/smartcontracts/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-623-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi

AUDIT="$REPO_ROOT/cw20-codeid-audits"
TAX_ON="$AUDIT/scripts/layer-b-tax-on.sh"
BLT="$AUDIT/scripts/layer-b-lt.sh"
OUT_JSON="${LAYER_B_TAX_ON_JSON:-$AUDIT/harness/layer-b-tax-on.json}"

run_docs() {
  set -euo pipefail
  test -f "$TAX_ON"
  test -f "$AUDIT/scripts/lib-tax-on.sh"
  test -f skills/AGENTS_CW20_CODE_ID_TAX_ON.md
  rg -q "C623-1" skills/AGENTS_CW20_CODE_ID_TAX_ON.md
  rg -q "layer-b-tax-on.sh" skills/AGENTS_CW20_CODE_ID_AUDIT.md
  rg -q "do not merge into B-lt" skills/AGENTS_CW20_CODE_ID_AUDIT.md \
    || rg -q "Do \*\*not\*\* merge into B-lt" skills/AGENTS_CW20_CODE_ID_AUDIT.md \
    || rg -q "not a substitute listing gate" skills/AGENTS_CW20_CODE_ID_AUDIT.md
  rg -q "verify-issue-623" AGENTS.md
  rg -q "verify-issue-623" docs/testing.md
  rg -q "C623-1" docs/contracts-security-audit.md
  rg -q "layer-b-tax-on.sh" "$AUDIT/harness/README.md"
  rg -q "tax-on suite" "$AUDIT/codeids/11611/REPORT.md"
  rg -q "tax-on suite" "$AUDIT/codeids/11619/REPORT.md"
  rg -q "layer-b-tax-on" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "AGENTS_CW20_CODE_ID_TAX_ON" skills/AGENTS_COMMUNITY_TAX_ROUTER.md
  rg -q "AGENTS_CW20_CODE_ID_TAX_ON" skills/AGENTS_COMMUNITY_TAX_AUTOLP.md
  rg -q "Do \*\*not\*\* RegisterListedPair" skills/AGENTS_CW20_CODE_ID_TAX_ON.md \
    || rg -q "Do not RegisterListedPair" skills/AGENTS_CW20_CODE_ID_TAX_ON.md
  # Ops must not whitelist from tax-on evidence.
  if rg -n "AddWhitelistedCodeId 8654" docs skills "$AUDIT"; then
    echo "docs tell operators to whitelist ALPHA 8654" >&2
    return 1
  fi
  if rg -n "AddWhitelistedCodeId 11611" "$AUDIT/scripts/layer-b-tax-on.sh" \
    "$AUDIT/scripts/lib-tax-on.sh" skills/AGENTS_CW20_CODE_ID_TAX_ON.md; then
    echo "tax-on docs/scripts tell operators to whitelist 11611" >&2
    return 1
  fi
}

run_blt_untouched() {
  set -euo pipefail
  # C623-1: generic B-lt must stay 1:1 and must not register listed pairs.
  if rg -n "register_listed_pair" "$BLT"; then
    echo "layer-b-lt.sh must not RegisterListedPair (C623-1)" >&2
    return 1
  fi
  if rg -n "debit >= amount|amount \+ tax|sell_bps" "$BLT"; then
    echo "layer-b-lt.sh must not special-case tax-on math (C589-5 / C623-1)" >&2
    return 1
  fi
  rg -q "layer_assert_debit" "$BLT"
  rg -q "Do not AddWhitelistedCodeId columbus-5" "$BLT" \
    || rg -q "never columbus-5" "$BLT"
}

run_known_bad() {
  (cd smartcontracts && cargo test -p cl8y-dex-tests --lib \
    mutant_a1_fot_breaks_one_to_one -- --quiet)
}

run_token_crates() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-token -p cl8y-community-tax-autolp \
    --offline --lib -- --test-threads=1)
}

run_601_docs_still() {
  set -euo pipefail
  rg -q "O601-1" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "does \*\*not\*\* register" skills/AGENTS_COMMUNITY_TAX_CW20.md \
    || rg -q "does not register" skills/AGENTS_COMMUNITY_TAX_CW20.md \
    || rg -q "B-lt does \*\*not\*\* register" "$AUDIT/codeids/11611/REPORT.md"
  rg -q "8654" skills/AGENTS_COMMUNITY_TAX_CW20.md
}

run_layer_b_tax_on() {
  set -euo pipefail
  chmod +x "$TAX_ON" "$AUDIT/scripts/lib-tax-on.sh" "$AUDIT/scripts/lib-layer-lt.sh"
  local has=1
  if timeout 20 make -s has-localterra >/dev/null 2>&1; then
    has=0
  fi
  if [[ "${LAYER_B_TAX_ON:-1}" == "1" ]]; then
    if [[ "$has" -ne 0 ]]; then
      echo "FAIL: LAYER_B_TAX_ON=1 but make has-localterra failed. Provision: make setup-cloud-localterra" >&2
      return 1
    fi
    LAYER_B_TAX_ON_JSON="$OUT_JSON" "$TAX_ON" || return 1
    test -f "$OUT_JSON" || return 1
    jq -e '.executed == true
      and .pair_direct_sell == true
      and .pair_direct_buy == true
      and .router_trader == true
      and .spoof_trader_negative == true
      and .missing_router_trader_fail_closed == true
      and .limit_one_to_one == true
      and .autolp_floor == true
      and .sell_tx != ""
      and .buy_tx != ""
      and .router_tx != ""
      and .limit_tx != ""
      and .skim_tx != ""' "$OUT_JSON" >/dev/null
    echo "Layer B tax-on executed (not a stub). JSON=$OUT_JSON"
    return 0
  fi
  if [[ "$has" -eq 0 ]]; then
    echo "LocalTerra is up; set LAYER_B_TAX_ON=1 (default) to execute the named suite."
    return 0
  fi
  echo "SKIP Layer B tax-on: make has-localterra (explicit; not a stub PASS). Cloud Agent: make setup-cloud-localterra"
}

echo ""
echo "── first pass ──"
run_step "docs: C623 + REPORT split + no 8654/11611 whitelist" run_docs
run_step "B-lt: still 1:1; no RegisterListedPair / tax math" run_blt_untouched
run_step "known-bad FoT mutant stays red" run_known_bad
run_step "crates: community-tax-token + AutoLP" run_token_crates
run_step "#601 docs still tax-off B-lt + no 8654 whitelist" run_601_docs_still
run_step "Layer B tax-on LocalTerra (fail-closed, not stub)" run_layer_b_tax_on

echo ""
echo "── retest ──"
run_step "retest docs: C623 + REPORT split" run_docs
run_step "retest B-lt untouched" run_blt_untouched
run_step "retest known-bad FoT red" run_known_bad
run_step "retest #601 docs tax-off B-lt" run_601_docs_still

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #623 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "==> GitLab #623 verification passed"
exit 0
