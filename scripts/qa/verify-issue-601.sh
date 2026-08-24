#!/usr/bin/env bash
# Verification for GitLab #601 — community tax store, #589 REPORT, factory
# whitelist, LocalTerra smoke.
#
# Invariants O601-1–O601-7: skills/AGENTS_COMMUNITY_TAX_CW20.md
# Child crate gate: make verify-issue-592 (T592-1–T592-12).
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
echo "  GitLab #601 — community tax store + listing + LocalTerra smoke"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
# Optimizer may leave smartcontracts/target root-owned; do not bind-mount cargo as root.
if [[ -e "$REPO_ROOT/smartcontracts/target" && ! -w "$REPO_ROOT/smartcontracts/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-601-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi
AUDIT="$REPO_ROOT/cw20-codeid-audits"
REPORT="$AUDIT/codeids/11611/REPORT.md"
PIN="9D33BF2539A9A5B2F13FD4B321CDBD0B0FD86D936D5D6BD6681955FA30210EC2"
FACTORY_C5="terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea"
LAUNCHER_C5="terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze"
LAUNCHER_ADMIN_C5="terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7"
LCD_C5="${VERIFY601_LCD:-https://terra-classic-lcd.publicnode.com}"

run_docs() {
  set -euo pipefail
  test -f "$REPORT"
  rg -qF '**GO**' "$REPORT"
  rg -q "$PIN" "$REPORT"
  rg -q "11611" "$REPORT"
  rg -q "NO-GO" "$AUDIT/codeids/community-tax-token/REPORT.md"
  pin="$(tr -d '[:space:]' < "$AUDIT/codeids/11611/wasm.sha256" | tr '[:lower:]' '[:upper:]')"
  test "$pin" = "$PIN"
  rg -q "O601-1" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "verify-issue-601" AGENTS.md
  rg -q "verify-issue-601" docs/testing.md
  rg -q "11611" docs/runbooks/cw20-whitelist-policy.md
  rg -q "Do \*\*not\*\* whitelist launcher" skills/AGENTS_COMMUNITY_TAX_CW20.md \
    || rg -q "Do not whitelist launcher" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "8654" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "O601-1" docs/contracts-security-audit.md
  rg -q "CreateToken" docs/contracts-terraclassic.md
  # Docs must not tell operators to whitelist sisters or ALPHA.
  if rg -n "AddWhitelistedCodeId 11612" docs skills cw20-codeid-audits; then
    echo "docs tell operators to whitelist launcher 11612" >&2
    return 1
  fi
  if rg -n "AddWhitelistedCodeId 8654" docs skills; then
    echo "docs tell operators to whitelist ALPHA 8654" >&2
    return 1
  fi
}

run_592() {
  make verify-issue-592
}

run_c5_whitelist() {
  set -euo pipefail
  # shellcheck source=scripts/lib/lcd-smart-query.sh
  source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
  local raw ids
  raw="$(lcd_smart_query_raw "$LCD_C5" "$FACTORY_C5" '{"get_whitelisted_code_ids":{}}')"
  ids="$(lcd_decode_smart_data "$raw")"
  echo "$ids" | jq -e '.code_ids | index(11626) != null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11611) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11612) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11613) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11614) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11620) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11621) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11622) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(8654) == null' >/dev/null
  echo "columbus-5 whitelist: $(echo "$ids" | jq -c '.code_ids')"
}

run_c5_launcher() {
  set -euo pipefail
  # shellcheck source=scripts/lib/lcd-smart-query.sh
  source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
  local raw
  raw="$(localterra_host_curl "${LCD_C5%/}/cosmwasm/wasm/v1/contract/${LAUNCHER_C5}" 2>/dev/null \
    || curl -fsS --max-time 30 "${LCD_C5%/}/cosmwasm/wasm/v1/contract/${LAUNCHER_C5}")"
  echo "$raw" | jq -e '.contract_info.code_id == "11622" or (.contract_info.code_id|tonumber) == 11622' >/dev/null
  local admin
  admin="$(echo "$raw" | jq -r '.contract_info.admin // empty')"
  [[ "$admin" == "$LAUNCHER_ADMIN_C5" ]]
  echo "columbus-5 launcher admin=$admin code=11622"
}

run_layer_b_lt() {
  # Make variables are not exported to the recipe env — call the script directly.
  CODE_ID=11611 LAYER_B_LT=1 "$REPO_ROOT/scripts/qa/verify-issue-589.sh"
}

run_smoke() {
  chmod +x "$REPO_ROOT/scripts/qa/localterra-community-tax-smoke.sh"
  "$REPO_ROOT/scripts/qa/localterra-community-tax-smoke.sh"
  local json="${VERIFY601_SMOKE_JSON:-/tmp/cl8y-601-smoke.json}"
  jq -e '.executed == true
    and .free_profile_create == true
    and .launcher_admin_cmm == true
    and .launcher_origin_set == true
    and .rogue_origin_null == true
    and .provide_one_to_one == true
    and .sell_extra_debit == true
    and .buy_outbound_split == true
    and .sku_unlock_50_ust1 == true
    and .sku_unlock_via_launcher == true
    and .settings_batch_50_ust1 == true
    and .mintcontrol_revoke_one_way == true
    and .paid_create_one_sku == true
    and .sku_second_unlock_via_launcher == true' "$json" >/dev/null
}

run_report_layer_notes() {
  set -euo pipefail
  rg -q "Layer A" "$REPORT"
  rg -q "Layer B" "$REPORT"
  test -f "$AUDIT/codeids/11611/layer-a-lcd.json"
  test -f "$AUDIT/codeids/11611/layer-b-lt.json"
  jq -e '.executed == true and .one_to_one == true' \
    "$AUDIT/codeids/11611/layer-a-lcd.json" >/dev/null
  jq -e '.executed == true and .round_trip_swap == true and .provide_liquidity == true' \
    "$AUDIT/codeids/11611/layer-b-lt.json" >/dev/null
}

echo ""
echo "── first pass ──"
run_step "docs: REPORT GO + O601 + no 11612/8654 whitelist" run_docs
run_step "child: make verify-issue-592" run_592
run_step "columbus-5: GetWhitelistedCodeIds includes 11626 (not 11611/sisters/8654)" run_c5_whitelist
run_step "columbus-5: launcher 11622 admin == DEX 2-of-3" run_c5_launcher

HAS_LT=1
if timeout 20 make -s has-localterra >/dev/null 2>&1; then
  HAS_LT=0
fi
if [[ "$HAS_LT" -eq 0 ]]; then
  run_step "CODE_ID=11611 LAYER_B_LT=1 make verify-issue-589" run_layer_b_lt
  run_step "LocalTerra smoke: free-profile + tax + invoices" run_smoke
  run_step "REPORT layer JSON executed on pinned 11611" run_report_layer_notes
else
  if [[ "${VERIFY601_REQUIRE_CHAIN:-1}" == "1" ]]; then
    bad "LocalTerra required (VERIFY601_REQUIRE_CHAIN=1) — make setup-cloud-localterra"
  else
    echo "  [SKIP] Layer B-lt + smoke (make has-localterra). Cloud Agent: make setup-cloud-localterra"
    RESULTS+=("SKIP  Layer B-lt + smoke")
  fi
fi

echo ""
echo "── retest ──"
run_step "retest docs: REPORT GO + O601" run_docs
run_step "retest child: make verify-issue-592" run_592

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #601 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "==> GitLab #601 verification passed"
exit 0
