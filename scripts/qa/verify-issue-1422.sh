#!/usr/bin/env bash
# Repeat funded reverse and receive regressions; unavailable checks stay explicit.
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
  for filter in issue1422 issue1389; do
    env CARGO_TARGET_DIR="$target" cargo test --manifest-path smartcontracts/Cargo.toml \
      -p cl8y-dex-router "$filter" -- --test-threads=1 || return 1
  done
  for filter in route_tax_1381_tests issue1390; do
    env CARGO_TARGET_DIR="$target" cargo test --manifest-path smartcontracts/Cargo.toml \
      -p cl8y-dex-tests "$filter" -- --test-threads=1 || return 1
  done
}
indexer() {
  local target="${INDEXER_CARGO_TARGET_DIR:-$PWD/indexer/target}"
  local debug="${INDEXER_CARGO_TEST_DEBUG:-${CARGO_PROFILE_TEST_DEBUG:-2}}"
  local incremental="${INDEXER_CARGO_INCREMENTAL:-${CARGO_INCREMENTAL:-1}}"
  for filter in gas_limit route_exact_output; do
    env CARGO_TARGET_DIR="$target" CARGO_PROFILE_DEV_DEBUG="$debug" \
      CARGO_PROFILE_TEST_DEBUG="$debug" CARGO_INCREMENTAL="$incremental" \
      cargo test --manifest-path indexer/Cargo.toml \
      --lib "$filter" -- --test-threads=1 || return 1
  done
  (cd indexer && env CARGO_TARGET_DIR="$target" CARGO_PROFILE_DEV_DEBUG="$debug" \
    CARGO_PROFILE_TEST_DEBUG="$debug" CARGO_INCREMENTAL="$incremental" \
    cargo test --test api_route_exact_output -- --test-threads=1)
}
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/pages/SwapPage.queryParams.test.tsx src/utils/cw20RouteSolveQuote.test.ts \
    src/utils/swapReceiveQuote.test.ts src/services/indexer/__tests__/client.test.ts \
    src/services/terraclassic/router.reverse.test.ts
}
source_checks() {
  python3 -m py_compile scripts/qa/localterra-route-tax-1381.py scripts/qa/localterra-funded-reverse-1422.py &&
    bash -n scripts/qa/verify-issue-1422.sh && git diff --check
}
chain() {
  local evidence="${VERIFY_ISSUE_1422_EVIDENCE:-/tmp/dex-1422-${1}.json}"
  local args=()
  if [[ -n "${VERIFY_ISSUE_1422_BASELINE_ARTIFACTS:-}" ]]; then
    args+=(--baseline-artifacts "$VERIFY_ISSUE_1422_BASELINE_ARTIFACTS")
  fi
  python3 scripts/qa/localterra-funded-reverse-1422.py \
    --container "${VERIFY_ISSUE_1422_CONTAINER:-cl8y-dex-issue1422-localterra}" \
    --node-home "${VERIFY_ISSUE_1422_NODE_HOME:-/home/terra/.terra}" \
    --artifacts "${VERIFY_ISSUE_1422_ARTIFACTS:-smartcontracts/artifacts}" \
    --probe "${VERIFY_ISSUE_1422_PROBE:-scripts/qa/fixtures/reverse-query-probe/target/wasm32-unknown-unknown/release/reverse_query_probe.wasm}" \
    --code-cache "${VERIFY_ISSUE_1422_CODE_CACHE:-/tmp/dex-1422-codes.json}" \
    --evidence-file "$evidence" "${args[@]}"
}
printf 'Revision: '; git rev-parse HEAD
date -u '+UTC: %Y-%m-%dT%H:%M:%SZ'
for pass in first retest; do
  for group in contracts indexer frontend; do
    reason_var="VERIFY_ISSUE_1422_SKIP_${group^^}_REASON"
    if [[ -n "${!reason_var:-}" ]]; then skip "$pass $group" "${!reason_var}"
    else run "$pass $group" "$group"; fi
  done
  run "$pass source checks" source_checks
  if [[ -n "${VERIFY_ISSUE_1422_SKIP_CHAIN_REASON:-}" ]]; then
    skip "$pass isolated 3M funded queries" "$VERIFY_ISSUE_1422_SKIP_CHAIN_REASON"
  else
    run "$pass isolated 3M funded queries" chain "$pass"
  fi
done
if [[ -z "${VERIFY_ISSUE_1422_BASELINE_ARTIFACTS:-}" ]]; then
  skip 'frozen before-fix regression' 'No independently reviewed pre-fix artifacts supplied'
fi
printf '\n%s\n' "${results[@]}"
printf 'PASS=%s FAIL=%s SKIP=%s\n' "$passed" "$failed" "$skipped"
if (( failed )); then echo 'Overall FAIL: executed checks failed.'; exit 1; fi
if (( skipped )); then echo 'Overall PASS with explicit skips; skipped criteria remain unverified.'
else echo 'Overall PASS'; fi
