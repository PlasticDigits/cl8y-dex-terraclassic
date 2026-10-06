#!/usr/bin/env bash
# Bounded greedy support: frozen reproduction is distinct from final acceptance.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
passed=0 failed=0 skipped=0
results=()
run() {
  local label="$1"; shift
  printf '\n[%s]\n' "$label"
  if "$@"; then passed=$((passed+1)); results+=("PASS $label")
  else failed=$((failed+1)); results+=("FAIL $label"); fi
}
skip() { skipped=$((skipped+1)); results+=("SKIP $1: $2"); }
contracts() {
  local target="${CONTRACTS_CARGO_TARGET_DIR:-$PWD/smartcontracts/target}"
  env CARGO_TARGET_DIR="$target" cargo test --locked --manifest-path smartcontracts/Cargo.toml \
    -p cl8y-dex-router -- --test-threads=1 || return 1
  env CARGO_TARGET_DIR="$target" cargo test --locked --manifest-path smartcontracts/Cargo.toml \
    -p cl8y-dex-tests route_tax_1381_tests -- --test-threads=1
}
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/pages/SwapPage.queryParams.test.tsx src/utils/cw20RouteSolveQuote.test.ts \
    src/utils/swapReceiveQuote.test.ts src/services/terraclassic/router.reverse.test.ts
}
source_checks() {
  python3 scripts/qa/test-localterra-reverse-1389.py || return 1
  python3 -m py_compile scripts/qa/localterra-reverse-1389.py \
    scripts/qa/localterra-funded-reverse-1422.py || return 1
  bash -n scripts/qa/verify-issue-1428.sh scripts/qa/verify-issue-1422.sh || return 1
  cargo fmt --manifest-path smartcontracts/Cargo.toml --all --check || return 1
  git diff --check
}
chain() {
  local pass="$1" artifacts="$2" version="$3" expectation="$4"
  python3 scripts/qa/localterra-reverse-1389.py \
    --container "${VERIFY_ISSUE_1428_CONTAINER:-cl8y-dex-issue1428-matrix}" \
    --node-home "${VERIFY_ISSUE_1428_NODE_HOME:-/home/terra/.terra}" \
    --artifacts "$artifacts" --router-version "$version" "$expectation" \
    --probe "${VERIFY_ISSUE_1428_PROBE:-scripts/qa/fixtures/reverse-query-probe/target/wasm32-unknown-unknown/release/reverse_query_probe.wasm}" \
    --code-cache "${VERIFY_ISSUE_1428_CODE_CACHE:-/tmp/dex-1428-codes.json}" \
    --evidence-file "${VERIFY_ISSUE_1428_EVIDENCE_DIR:-/tmp/dex-1428}/matrix-${pass}.json"
}
printf 'Revision: '; git rev-parse HEAD
date -u '+UTC: %Y-%m-%dT%H:%M:%SZ'
if [[ -n "${VERIFY_ISSUE_1428_SKIP_BASELINE_REASON:-}" ]]; then
  skip 'integrated frozen reproduction' "$VERIFY_ISSUE_1428_SKIP_BASELINE_REASON"
elif [[ -n "${VERIFY_ISSUE_1428_BASELINE_ARTIFACTS:-}" ]]; then
  run 'integrated frozen reproduction' chain baseline "$VERIFY_ISSUE_1428_BASELINE_ARTIFACTS" \
    1.4.0 --expect-populated-gas-failure
else
  skip 'integrated frozen reproduction' 'No independently reviewed integrated baseline artifacts supplied'
fi
for pass in first retest; do
  for group in contracts frontend; do
    reason_var="VERIFY_ISSUE_1428_SKIP_${group^^}_REASON"
    if [[ -n "${!reason_var:-}" ]]; then skip "$pass $group" "${!reason_var}"
    else run "$pass $group" "$group"; fi
  done
  run "$pass source and harness checks" source_checks
  if [[ -n "${VERIFY_ISSUE_1428_SKIP_CHAIN_REASON:-}" ]]; then
    skip "$pass isolated populated matrix" "$VERIFY_ISSUE_1428_SKIP_CHAIN_REASON"
  else
    run "$pass isolated populated matrix" chain "$pass" \
      "${VERIFY_ISSUE_1428_ARTIFACTS:-smartcontracts/artifacts}" 1.5.0 \
      --expect-four-hop-greedy-rejection
  fi
done
printf '\n%s\n' "${results[@]}"
printf 'PASS=%s FAIL=%s SKIP=%s\n' "$passed" "$failed" "$skipped"
if (( failed )); then echo 'Overall FAIL: executed checks failed.'; exit 1; fi
if (( skipped )); then echo 'Overall PASS with explicit skips; skipped criteria remain unverified.'
else echo 'Overall PASS'; fi
