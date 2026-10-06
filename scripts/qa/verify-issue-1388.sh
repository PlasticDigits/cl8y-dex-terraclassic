#!/usr/bin/env bash
# Honest source verification and retest; live evidence is scoped separately.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
PASS=0 FAIL=0 SKIP=0
declare -a RESULTS=()
run() {
  local label="$1"; shift
  if "$@"; then PASS=$((PASS+1)); RESULTS+=("PASS $label")
  else FAIL=$((FAIL+1)); RESULTS+=("FAIL $label"); fi
}
contracts() {
  env CARGO_TARGET_DIR="${CONTRACTS_CARGO_TARGET_DIR:-$PWD/smartcontracts/target}" \
    cargo test --manifest-path smartcontracts/Cargo.toml \
      -p cl8y-community-tax-token -p cl8y-dex-router -p cl8y-dex-tests \
      issue1388 -- --test-threads=1
}
indexer() {
  env CARGO_TARGET_DIR="${INDEXER_CARGO_TARGET_DIR:-$PWD/indexer/target}" \
    cargo test --manifest-path indexer/Cargo.toml --lib trading_tax_discovery -- --test-threads=1
}
rollout() {
  python3 scripts/qa/test-route-tax-rollout.py || return 1
  python3 scripts/qa/test-localterra-route-tax-1388.py
}
for pass in first retest; do
  for group in contracts indexer rollout; do
    reason_var="VERIFY_ISSUE_1388_SKIP_${group^^}_REASON"
    if [[ -n "${!reason_var:-}" ]]; then
      SKIP=$((SKIP+1)); RESULTS+=("SKIP $pass $group: ${!reason_var}")
    else run "$pass $group" "$group"; fi
  done
done
run "contract format" cargo fmt --manifest-path smartcontracts/Cargo.toml --all --check
run "indexer format" python3 scripts/qa/check-issue-1381-format.py
run "documentation" python3 scripts/qa/check-issue-1388-docs.py
run "rollout source and retest" make verify-route-tax-rollout
run "diff whitespace" git diff --check
for group in CHAIN LIVE; do
  command_var="VERIFY_ISSUE_1388_${group}_COMMAND"
  reason_var="VERIFY_ISSUE_1388_SKIP_${group}_REASON"
  if [[ -n "${!command_var:-}" ]]; then
    run "$group evidence" bash -c "${!command_var}"
  else
    SKIP=$((SKIP+1))
    RESULTS+=("SKIP $group: ${!reason_var:-No execution command supplied; follow docs/qa/issue-1388/README.md}")
  fi
done
printf '\n%s\n' "${RESULTS[@]}"
printf '\nPASS=%s FAIL=%s SKIP=%s\n' "$PASS" "$FAIL" "$SKIP"
if (( FAIL )); then echo 'Overall FAIL: executed checks failed.'; exit 1; fi
if (( SKIP )); then echo 'Overall PASS with explicit skips; skipped criteria remain unverified.'
else echo 'Overall PASS: all checks passed.'; fi
