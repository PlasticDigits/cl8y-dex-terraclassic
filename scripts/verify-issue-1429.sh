#!/usr/bin/env bash
# Source verification: inventory/migration/deployment receipts remain separate.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
PASS=0 FAIL=0 SKIP=0
declare -a RESULTS=()
run_or_skip() {
  local label="$1" group="$2"; shift 2
  local reason_var="VERIFY_ISSUE_1429_SKIP_${group}_REASON"
  if [[ -n "${!reason_var:-}" ]]; then
    SKIP=$((SKIP + 1)); RESULTS+=("SKIP $label: ${!reason_var}")
  elif "$@"; then PASS=$((PASS + 1)); RESULTS+=("PASS $label")
  else FAIL=$((FAIL + 1)); RESULTS+=("FAIL $label"); fi
}
contracts_cargo() {
  local command="$1"; shift
  env CARGO_HOME="${CONTRACTS_CARGO_HOME:-${CARGO_HOME:-$HOME/.cargo}}" \
    CARGO_TARGET_DIR="${CONTRACTS_CARGO_TARGET_DIR:-$PWD/smartcontracts/target}" \
    cargo "$command" --manifest-path smartcontracts/Cargo.toml "$@"
}
focused() {
  contracts_cargo test -p cl8y-community-tax-token -- --test-threads=1 || return 1
  contracts_cargo test -p cl8y-community-tax-autolp -- --test-threads=1
}
workspace() { contracts_cargo test --workspace -- --test-threads=1; }
formatting() { cargo fmt --manifest-path smartcontracts/Cargo.toml --all --check; }
docs() {
  python3 scripts/qa/test_inventory_autolp_1429.py || return 1
  python3 - <<'PY'
from pathlib import Path
required = {
    'AGENTS.md': ['AGENTS_AUTOLP_POLICY_RECOVERY.md', 'verify-issue-1429'],
    'skills/AGENTS_AUTOLP_POLICY_RECOVERY.md': [f'LP1429-{i}' for i in range(1, 9)],
    'skills/AGENTS_COMMUNITY_TAX_AUTOLP.md': ['AGENTS_AUTOLP_POLICY_RECOVERY.md', 'verify-issue-1429'],
    'skills/AGENTS_NET_EXECUTION_FLOORS.md': ['AGENTS_AUTOLP_POLICY_RECOVERY.md'],
    'docs/contracts-terraclassic.md': ['refresh_autolp_policy', 'AGENTS_AUTOLP_POLICY_RECOVERY.md'],
    'docs/qa/issue-1429/README.md': ['AC1', 'AC2', 'AC3', 'AC4', 'AC5', 'AC6', 'AC7', 'PASS WITH SKIPS', 'FAIL'],
}
for filename, terms in required.items():
    body = Path(filename).read_text()
    for term in terms:
        assert term in body, f'{filename}: missing {term}'
assert 'mod autolp_recovery_tests;' in Path('smartcontracts/contracts/community-tax-token/src/multitest.rs').read_text()
assert Path('smartcontracts/contracts/community-tax-token/src/autolp_recovery_multitest.rs').is_file()
print('Issue #1429 invariant and acceptance crosslinks: PASS')
PY
}
printf 'Issue #1429 source verification at %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
printf 'Revision: %s (working tree may include uncommitted changes)\n' "$(git rev-parse HEAD)"
for pass in first retest; do
  run_or_skip "$pass recovery, no-op and economic rollback" CONTRACTS focused
done
run_or_skip 'full contract workspace' FULL_CONTRACTS workspace
run_or_skip 'Rust formatting' FORMAT formatting
run_or_skip 'inventory unit tests and documentation crosslinks' DOCS docs
printf '\n%s\n' "${RESULTS[@]}"
printf '\nResults: %s PASS, %s FAIL, %s SKIP\n' "$PASS" "$FAIL" "$SKIP"
if (( FAIL > 0 )); then printf 'Overall: FAIL\n'; exit 1; fi
if (( SKIP > 0 )); then printf 'Overall: PASS WITH SKIPS (source scope only)\n'
else printf 'Overall: PASS (source scope only)\n'; fi
