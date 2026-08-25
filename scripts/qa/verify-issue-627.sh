#!/usr/bin/env bash
# Verification for GitLab #627 — columbus-5 CW20 code 3 adopt + factory-list NO-GO.
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
echo "  GitLab #627 — CW20 code 3 investigate (adopt + list NO-GO)"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
if [[ -e "$REPO_ROOT/smartcontracts/target" && ! -w "$REPO_ROOT/smartcontracts/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-627-sc-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi

AUDIT="$REPO_ROOT/cw20-codeid-audits"
PIN_EXPECT="F9B4AB2202A5110B653E7DFE3E413B336D14234ED447D882E1D9BD5512B44891"

run_tree() {
  set -euo pipefail
  test -f "$AUDIT/codeids/3/REPORT.md"
  test -f "$AUDIT/codeids/3/wasm.sha256"
  test -f "$AUDIT/codeids/3/meta.json"
  test -f "$AUDIT/codeids/3/census.json"
  test -f "$AUDIT/codeids/3/decomp/fingerprint.json"
  test -f skills/AGENTS_CW20_CODE_ID_3.md
  local pin
  pin="$(tr -d '[:space:]' < "$AUDIT/codeids/3/wasm.sha256" | tr '[:lower:]' '[:upper:]')"
  [[ "$pin" == "$PIN_EXPECT" ]] || { echo "pin $pin != $PIN_EXPECT" >&2; return 1; }
}

run_report_nogo() {
  set -euo pipefail
  local r="$AUDIT/codeids/3/REPORT.md"
  rg -q "NO-GO" "$r"
  rg -q "CanonicalAddr" "$r"
  rg -q "AdoptLegacyLayout" "$r"
  rg -q "interface_version_7" "$r"
  rg -q "34" "$r"
  # Must not declare factory or adopt GO.
  if rg -n "^\*\*GO\*\* for factory" "$r"; then
    echo "REPORT must not GO factory list" >&2
    return 1
  fi
}

run_fingerprint() {
  set -euo pipefail
  local f="$AUDIT/codeids/3/decomp/fingerprint.json"
  python3 - "$f" <<'PY'
import json, sys
j = json.load(open(sys.argv[1], encoding="utf-8"))
h = j["hits"]
assert h.get("cw20_legacy") is True, h
assert h.get("interface_version_7") is True, h
assert h.get("addr_canonicalize") is True, h
assert h.get("tax_map") is False, h
assert h.get("UpdateTaxMap") is False, h
PY
}

run_no_enable() {
  set -euo pipefail
  rg -q "DEFAULT_COMMUNITY_MIGRATE_CODE_IDS = \\[6036, 10184, 8266, 8654\\]" \
    frontend-dapp/src/utils/communityTaxMigrate.ts
  if rg -n "DEFAULT_COMMUNITY_MIGRATE_CODE_IDS = \\[[^]]*\\b3\\b" frontend-dapp/src/utils/communityTaxMigrate.ts; then
    echo "default migrate list must not include 3" >&2
    return 1
  fi
  if rg -n "AddWhitelistedCodeId 3" smartcontracts/contracts scripts/upgrade*.sh 2>/dev/null; then
    echo "must not AddWhitelistedCodeId 3" >&2
    return 1
  fi
}

run_docs() {
  set -euo pipefail
  rg -q "C627-1" skills/AGENTS_CW20_CODE_ID_3.md
  rg -q "S3-code3" skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md
  rg -q "AdoptLegacyLayout" skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md
  rg -q "AGENTS_CW20_CODE_ID_3" skills/AGENTS_CW20_CODE_ID_AUDIT.md
  rg -q "verify-issue-627" AGENTS.md
  rg -q "verify-issue-627" docs/testing.md
  rg -q "AdoptLegacyLayout" docs/contracts-terraclassic.md
  rg -q "codeids/3/REPORT.md" docs/contracts-terraclassic.md
  rg -q "code 3" docs/runbooks/cw20-whitelist-policy.md
  rg -q "Do not append 3" docs/frontend.md
  rg -q "codeids/3/REPORT.md" cw20-codeid-audits/README.md
}

run_crates() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-token --offline --lib adopt -- --test-threads=1)
}

run_frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/communityTaxMigrate.test.ts
}

echo ""
echo "── first pass ──"
run_step "tree: pin + REPORT + census" run_tree
run_step "REPORT: dual-track NO-GO + layout" run_report_nogo
run_step "fingerprint: cw20_legacy + v7 + no tax_map" run_fingerprint
run_step "policy: 3 not in migrate env / factory scripts" run_no_enable
run_step "docs: C627 + migrate skill + AGENTS" run_docs
run_step "crates: adopt guard + honest adopt" run_crates
run_step "frontend: code 3 unlisted" run_frontend

echo ""
echo "── retest ──"
run_step "retest tree: pin + REPORT + census" run_tree
run_step "retest REPORT: dual-track NO-GO + layout" run_report_nogo
run_step "retest fingerprint: cw20_legacy + v7 + no tax_map" run_fingerprint
run_step "retest policy: 3 not in migrate env / factory scripts" run_no_enable
run_step "retest docs: C627 + migrate skill + AGENTS" run_docs
run_step "retest crates: adopt guard + honest adopt" run_crates
run_step "retest frontend: code 3 unlisted" run_frontend

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #627 verification passed"
