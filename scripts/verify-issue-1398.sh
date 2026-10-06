#!/usr/bin/env bash
# Source/test verification for #1398; deployed-code acceptance is separate.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
PASS=0 FAIL=0 SKIP=0
declare -a RESULTS=()
run() {
  local label="$1"; shift
  printf '\n[%s]\n' "$label"
  if "$@"; then PASS=$((PASS + 1)); RESULTS+=("PASS $label")
  else FAIL=$((FAIL + 1)); RESULTS+=("FAIL $label"); fi
}
run_or_skip() {
  local label="$1" group="$2"; shift 2
  local reason_var="VERIFY_ISSUE_1398_SKIP_${group}_REASON"
  if [[ -n "${!reason_var:-}" ]]; then
    SKIP=$((SKIP + 1)); RESULTS+=("SKIP $label: ${!reason_var}")
  else run "$label" "$@"; fi
}
contracts_cargo() {
  local command="$1"; shift
  env CARGO_HOME="${CONTRACTS_CARGO_HOME:-${CARGO_HOME:-$HOME/.cargo}}" \
    CARGO_TARGET_DIR="${CONTRACTS_CARGO_TARGET_DIR:-$PWD/smartcontracts/target}" \
    cargo "$command" --manifest-path smartcontracts/Cargo.toml "$@"
}
focused() {
  contracts_cargo test -p dex-common hook -- --test-threads=1 || return 1
  contracts_cargo test -p cl8y-dex-pair -p cl8y-dex-factory hook_bounds -- --test-threads=1 || return 1
  contracts_cargo test -p cl8y-dex-router issue1389 -- --test-threads=1 || return 1
  contracts_cargo test -p cl8y-community-tax-autolp -- --test-threads=1 || return 1
  contracts_cargo test -p cl8y-dex-tests issue_1398 -- --test-threads=1
}
workspace() { contracts_cargo test --workspace -- --test-threads=1; }
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/utils/swapMaxSpread.test.ts src/utils/cw20RouteSolveQuote.test.ts \
    src/utils/directHybridQuote.test.ts src/services/terraclassic/router.test.ts \
    src/services/terraclassic/router.reverse.test.ts \
    src/services/terraclassic/__tests__/swapRoutePreflight.test.ts \
    src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx \
    src/pages/SwapPage.test.tsx
}
formatting() { cargo fmt --manifest-path smartcontracts/Cargo.toml --all --check; }
docs() {
  python3 - <<'PY'
from pathlib import Path
required = {
    'AGENTS.md': ['AGENTS_NET_EXECUTION_FLOORS.md', 'verify-issue-1398'],
    'docs/integrators.md': ['#1398', 'post-hook', 'min_return', 'AGENTS_NET_EXECUTION_FLOORS.md'],
    'docs/contracts-security-audit.md': ['NE1398', 'verify-issue-1398'],
    'skills/AGENTS_NET_EXECUTION_FLOORS.md': [f'NE1398-{i}' for i in range(1, 11)],
    'skills/AGENTS_MAX_SPREAD_HYBRID.md': ['AGENTS_NET_EXECUTION_FLOORS.md'],
    'skills/AGENTS_HYBRID_QUOTING.md': ['AGENTS_NET_EXECUTION_FLOORS.md', 'post-hook'],
    'skills/AGENTS_HOOK_COMMISSION.md': ['AGENTS_NET_EXECUTION_FLOORS.md'],
    'skills/AGENTS_COMMUNITY_TAX_AUTOLP.md': ['AGENTS_NET_EXECUTION_FLOORS.md', 'verify-issue-1398'],
    'docs/qa/issue-1398/README.md': ['AC1', 'AC2', 'AC3', 'AC4', 'PASS WITH SKIPS', 'FAIL'],
}
for filename, terms in required.items():
    body = Path(filename).read_text()
    for term in terms:
        assert term in body, f'{filename}: missing {term}'
# A renamed/deleted module must not turn a focused filter into a zero-test pass.
assert 'mod issue_1398_tests;' in Path('smartcontracts/tests/src/lib.rs').read_text()
assert Path('smartcontracts/tests/src/issue_1398_tests.rs').is_file()
print('Issue #1398 invariant and acceptance crosslinks: PASS')
PY
}
printf 'Issue #1398 source verification at %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
printf 'Revision: %s (working tree may include uncommitted changes)\n' "$(git rev-parse HEAD)"
for pass in first retest; do
  run_or_skip "$pass focused contracts" CONTRACTS focused
  run_or_skip "$pass frontend quote/signing" FRONTEND frontend
done
run_or_skip 'full contract workspace' FULL_CONTRACTS workspace
run_or_skip 'Rust formatting' FORMAT formatting
run_or_skip 'documentation and invariant crosslinks' DOCS docs
printf '\n%s\n' "${RESULTS[@]}"
printf '\nResults: %s PASS, %s FAIL, %s SKIP\n' "$PASS" "$FAIL" "$SKIP"
if (( FAIL > 0 )); then printf 'Overall: FAIL\n'; exit 1; fi
if (( SKIP > 0 )); then printf 'Overall: PASS WITH SKIPS (source scope only)\n'
else printf 'Overall: PASS (source scope only)\n'; fi
