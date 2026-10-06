#!/usr/bin/env bash
# Source-level verification for #1399; deployment evidence is recorded separately.
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
  local reason_var="VERIFY_ISSUE_1399_SKIP_${group}_REASON"
  if [[ -n "${!reason_var:-}" ]]; then
    SKIP=$((SKIP + 1)); RESULTS+=("SKIP $label: ${!reason_var}")
  else run "$label" "$@"; fi
}
contracts() {
  contracts_cargo test -p dex-common limit_placement -- --test-threads=1 || return 1
  contracts_cargo test -p dex-common expired_limit_park_reason -- --test-threads=1 || return 1
  contracts_cargo test -p cl8y-dex-pair economic_ -- --test-threads=1 || return 1
  contracts_cargo test -p cl8y-dex-tests unfillable -- --test-threads=1 || return 1
  contracts_cargo test -p cl8y-dex-tests zero_cost -- --test-threads=1 || return 1
  contracts_cargo test -p cl8y-dex-tests expired_prefix -- --test-threads=1
}
full_contracts() { contracts_cargo test --workspace -- --test-threads=1; }
indexer() { indexer_cargo test --lib db_orderbook_sim -- --test-threads=1; }
db() { indexer_cargo test --test api_route_solve_db_hybrid -- --test-threads=1; }
formatting() {
  cargo fmt --manifest-path smartcontracts/Cargo.toml --all --check || return 1
  rustfmt --edition 2021 --config skip_children=true --check \
    indexer/src/api/db_orderbook_sim.rs indexer/tests/api_route_solve_db_hybrid.rs
}
docs() {
  python3 - <<'PY'
from pathlib import Path
required = {
    'AGENTS.md': ['AGENTS_UNFILLABLE_LIMIT_ORDERS.md', 'verify-issue-1399'],
    'docs/limit-orders.md': ['#1399', 'unfillable', 'AGENTS_UNFILLABLE_LIMIT_ORDERS.md'],
    'docs/contracts-security-audit.md': ['UF1399', 'verify-issue-1399'],
    'docs/integrators.md': ['unfillable', 'AGENTS_UNFILLABLE_LIMIT_ORDERS.md'],
    'docs/qa/issue-1399/README.md': ['AC1', 'AC2', 'AC3', 'AC4', 'PASS WITH SKIPS', 'FAIL'],
    'skills/AGENTS_UNFILLABLE_LIMIT_ORDERS.md': [f'UF1399-{i}' for i in range(1, 11)],
}
for filename, terms in required.items():
    body = Path(filename).read_text()
    for term in terms:
        assert term in body, f'{filename}: missing {term}'
print('Issue #1399 invariants, acceptance criteria and skip policy crosslinks: PASS')
PY
}
# Keep manifest arguments before the cargo test '--' separator.
contracts_cargo() {
  local command="$1"; shift
  env CARGO_HOME="${CONTRACTS_CARGO_HOME:-${CARGO_HOME:-$HOME/.cargo}}" \
    CARGO_TARGET_DIR="${CONTRACTS_CARGO_TARGET_DIR:-$PWD/smartcontracts/target}" \
    cargo "$command" --manifest-path smartcontracts/Cargo.toml "$@"
}
indexer_cargo() {
  local command="$1"; shift
  local target_dir="${INDEXER_CARGO_TARGET_DIR:-$PWD/indexer/target}"
  # DB test fixtures load .env relative to their working directory. Stay inside
  # this worktree's indexer so its isolated database configuration is honored.
  (cd indexer && env CARGO_HOME="${INDEXER_CARGO_HOME:-${CARGO_HOME:-$HOME/.cargo}}" \
    CARGO_TARGET_DIR="$target_dir" cargo "$command" "$@")
}
printf 'Issue #1399 source verification at %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
printf 'Revision: %s (working tree may include uncommitted implementation)\n' "$(git rev-parse HEAD)"
for pass in first retest; do
  run_or_skip "$pass contracts" CONTRACTS contracts
  run_or_skip "$pass indexer" INDEXER indexer
done
run_or_skip 'full contract workspace' FULL_CONTRACTS full_contracts
run_or_skip 'DB hybrid route integration' DB db
run_or_skip 'Rust formatting' FORMAT formatting
run_or_skip 'documentation and invariant crosslinks' DOCS docs
printf '\n%s\n' "${RESULTS[@]}"
printf '\nResults: %s PASS, %s FAIL, %s SKIP\n' "$PASS" "$FAIL" "$SKIP"
if (( FAIL > 0 )); then printf 'Overall: FAIL\n'; exit 1; fi
if (( SKIP > 0 )); then printf 'Overall: PASS WITH SKIPS (source scope only)\n'
else printf 'Overall: PASS (source scope only)\n'; fi
