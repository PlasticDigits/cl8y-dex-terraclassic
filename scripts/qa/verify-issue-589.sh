#!/usr/bin/env bash
# Verification for GitLab #589 — CW20 code-id audit harness (decomp + exploit suite).
#
# Refs: cw20-codeid-audits/PROCEDURE.md
#       skills/AGENTS_CW20_CODE_ID_AUDIT.md
#       docs/runbooks/cw20-whitelist-policy.md
#
# CODE_ID=8266  also fetch/fingerprint that LCD wasm (network).
# LAYER_B_LT=1  require LocalTerra store path (otherwise explicit skip).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }

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
echo "  GitLab #589 — CW20 code-id audit harness"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
AUDIT="$REPO_ROOT/cw20-codeid-audits"
chmod +x "$AUDIT/scripts/fetch-lcd-wasm.sh" \
  "$AUDIT/scripts/decompile-wasm.sh" \
  "$AUDIT/scripts/fingerprint-wasm.sh"

# --- tree ---
run_tree() {
  set -euo pipefail
  test -f "$AUDIT/README.md"
  test -f "$AUDIT/PROCEDURE.md"
  test -f "$AUDIT/CATALOG.md"
  test -f "$AUDIT/report-template.md"
  test -f "$AUDIT/scripts/fetch-lcd-wasm.sh"
  test -f "$AUDIT/scripts/decompile-wasm.sh"
  test -f "$AUDIT/harness/README.md"
  test -d "$AUDIT/codeids/8266"
  test -f "$AUDIT/codeids/8266/REPORT.md"
  test -f "$AUDIT/codeids/8266/wasm.sha256"
  test -f skills/AGENTS_CW20_CODE_ID_AUDIT.md
}

# --- catalogue completeness ---
run_catalog_ids() {
  set -euo pipefail
  local f="$AUDIT/CATALOG.md"
  for id in A1 A30 B1 B15 C1 C7 D1 D22 E1 E15 CH1 CH18 G1 G9; do
    rg -q "^\\| ${id} " "$f" || { echo "missing catalogue row $id" >&2; return 1; }
  done
  rg -q "catalog F1" "$f" || rg -q "| F1 " "$f"
  rg -q "d-xo/weird-erc20" "$f"
  rg -q "CWA-2024-002" "$f"
  rg -q "tx.origin" "$f"
}

# --- policy: hash-equal rebuild is not the gate ---
run_policy_docs() {
  set -euo pipefail
  rg -q "cw20-codeid-audits" docs/runbooks/cw20-whitelist-policy.md
  rg -q "PROCEDURE.md" docs/runbooks/cw20-whitelist-policy.md
  # Gate language must not require byte-identical rebuild.
  if rg -n "remaining gate is an optimizer rebuild" docs/runbooks/cw20-whitelist-policy.md; then
    echo "policy still treats optimizer rebuild as the remaining gate" >&2
    return 1
  fi
  rg -q "optional appendix" docs/runbooks/cw20-whitelist-policy.md
  rg -q "AGENTS_CW20_CODE_ID_AUDIT" AGENTS.md
  rg -q "verify-issue-589" AGENTS.md
  rg -q "verify-issue-590" AGENTS.md
  rg -q "layer-a-lcd.sh" "$AUDIT/harness/README.md"
  rg -q "cw20-codeid-audits" docs/runbooks/cw20-code-id-ops.md
  rg -q "codeids/8266/REPORT.md" skills/AGENTS_CW20_CODE_ID_PIN.md
  rg -q "cw20-codeid-audits" audits/CW20-8266-581.md
  rg -q "SEC-D06" docs/exploit-replay-matrix.md
  test -f "$AUDIT/codeids/8266/REPORT.md"
}

run_fetch_selftest() {
  "$AUDIT/scripts/fetch-lcd-wasm.sh" --self-test
}

run_fingerprint_selftest() {
  "$AUDIT/scripts/fingerprint-wasm.sh" --self-test
}

run_decomp_selftest() {
  set -euo pipefail
  if command -v wasm2wat >/dev/null 2>&1; then
    "$AUDIT/scripts/decompile-wasm.sh" --self-test
  else
    if "$AUDIT/scripts/decompile-wasm.sh" --self-test; then
      echo "decompile --self-test succeeded without wabt (C2 broken)" >&2
      return 1
    fi
    echo "OK: decompile fail-closed without wabt (install wabt to decompile LCD wasm)"
  fi
}

run_c2_script_guard() {
  rg -q "do not skip decomp" "$AUDIT/scripts/decompile-wasm.sh"
  rg -q "FAIL closed" "$AUDIT/scripts/fetch-lcd-wasm.sh" || rg -q "C1" "$AUDIT/scripts/fetch-lcd-wasm.sh"
}

run_c6_no_secrets() {
  set -euo pipefail
  if rg -n -i "mnemonic|BEGIN OPENSSH|private_key" "$AUDIT/codeids" "$AUDIT/fixtures" 2>/dev/null | rg -v "No live keys"; then
    echo "possible secret in audit tree (C6)" >&2
    return 1
  fi
  return 0
}

run_layer_ab_mt() {
  (cd smartcontracts && cargo test -p cl8y-dex-tests --lib cw20_codeid_harness -- --quiet)
}

run_known_bad_still_red() {
  (cd smartcontracts && cargo test -p cl8y-dex-tests --lib fot -- --quiet)
}

run_pin_8266() {
  set -euo pipefail
  local pin
  pin="$(tr -d '[:space:]' < "$AUDIT/codeids/8266/wasm.sha256" | tr '[:lower:]' '[:upper:]')"
  [[ "$pin" == "953AD60CF6D8C9631B99ADC84C3ABF4083815743F86FF81B2A422FDFDF5F95C0" ]]
  rg -q "953AD60C" "$AUDIT/codeids/8266/REPORT.md"
  rg -q "NO-GO" "$AUDIT/codeids/8266/REPORT.md"
}

run_layer_b_lt() {
  set -euo pipefail
  local has=1
  if timeout 20 make -s has-localterra >/dev/null 2>&1; then
    has=0
  fi
  chmod +x "$AUDIT/scripts/layer-a-lcd.sh" "$AUDIT/scripts/layer-b-lt.sh"
  if [[ "${LAYER_B_LT:-}" == "1" ]]; then
    if [[ "$has" -ne 0 ]]; then
      echo "FAIL: LAYER_B_LT=1 but make has-localterra failed. Provision: make setup-cloud-localterra" >&2
      return 1
    fi
    local id="${CODE_ID:-}"
    if [[ -z "$id" ]]; then
      echo "FAIL: LAYER_B_LT=1 requires CODE_ID so pinned token.wasm is executed (not a stub)" >&2
      return 1
    fi
    if [[ ! -f "$AUDIT/codeids/$id/token.wasm" ]]; then
      "$AUDIT/scripts/fetch-lcd-wasm.sh" "$id"
    fi
    "$AUDIT/scripts/layer-a-lcd.sh" "$id"
    "$AUDIT/scripts/layer-b-lt.sh" "$id"
    test -f "$AUDIT/codeids/$id/layer-a-lcd.json"
    test -f "$AUDIT/codeids/$id/layer-b-lt.json"
    jq -e '.executed == true and .one_to_one == true' "$AUDIT/codeids/$id/layer-a-lcd.json" >/dev/null
    jq -e '.executed == true and .one_to_one_into_pair == true' "$AUDIT/codeids/$id/layer-b-lt.json" >/dev/null
    echo "Layer A-lcd + B-lt executed pinned LCD wasm $id (see layer-*-*.json)."
    return 0
  fi
  if [[ "$has" -eq 0 ]]; then
    echo "LocalTerra is up; B-mt still used for this target. Set LAYER_B_LT=1 CODE_ID=… to store LCD wasm."
    return 0
  fi
  echo "SKIP Layer B-lt: make has-localterra (explicit; B-mt ran). Cloud Agent: make setup-cloud-localterra"
}

run_code_id_lcd() {
  local id="${CODE_ID:-}"
  if [[ -z "$id" ]]; then
    echo "SKIP LCD fetch (CODE_ID unset; CI path uses cached pin + self-tests)"
    return 0
  fi
  "$AUDIT/scripts/fetch-lcd-wasm.sh" "$id"
  "$AUDIT/scripts/fingerprint-wasm.sh" "$id"
  if command -v wasm2wat >/dev/null 2>&1; then
    "$AUDIT/scripts/decompile-wasm.sh" "$id"
  else
    echo "FAIL: CODE_ID=$id requires wabt for decomp (C2). apt install wabt" >&2
    return 1
  fi
}

echo ""
echo "── first pass ──"
run_step "tree: PROCEDURE CATALOG template scripts codeids/8266" run_tree
run_step "catalog: required rows + citations" run_catalog_ids
run_step "docs: decomp+suite gate; rebuild is appendix" run_policy_docs
run_step "fetch: pin match / mismatch / truncated / non-wasm (C1 G9)" run_fetch_selftest
run_step "fingerprint self-test" run_fingerprint_selftest
run_step "decompile: wabt present or fail-closed (C2)" run_decomp_selftest
run_step "scripts: C1/C2 fail-closed strings" run_c2_script_guard
run_step "C6: no mnemonic/key in codeids/" run_c6_no_secrets
run_step "8266 pin + REPORT verdict present" run_pin_8266
run_step "Layer A+B multi-test (mintable + mutants)" run_layer_ab_mt
run_step "known-bad FoT 1:1 and P2 stay red" run_known_bad_still_red
run_step "Layer B-lt LocalTerra (explicit skip or require)" run_layer_b_lt
run_step "optional CODE_ID LCD fetch+decomp" run_code_id_lcd

echo ""
echo "── retest ──"
run_step "retest fetch self-test" run_fetch_selftest
run_step "retest Layer A+B multi-test" run_layer_ab_mt
run_step "retest known-bad FoT red" run_known_bad_still_red

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #589 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
exit 0
