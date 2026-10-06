#!/usr/bin/env bash
# Synthetic checks first, explicit unavailable-scope skips, failures stay failures.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
passed=0 failed=0 skipped=0
results=()
run() {
  local label="$1"; shift
  if "$@"; then passed=$((passed+1)); results+=("PASS $label")
  else failed=$((failed+1)); results+=("FAIL $label"); fi
}
skip() { skipped=$((skipped+1)); results+=("SKIP $1: $2"); }
contracts() {
  if [[ -n "${VERIFY_ISSUE_1389_CONTRACTS_COMMAND:-}" ]]; then
    bash -c "$VERIFY_ISSUE_1389_CONTRACTS_COMMAND"
  else
    (cd smartcontracts && cargo test --workspace issue1389 -- --test-threads=1)
  fi
}
indexer() {
  if [[ -n "${VERIFY_ISSUE_1389_INDEXER_COMMAND:-}" ]]; then
    bash -c "$VERIFY_ISSUE_1389_INDEXER_COMMAND"
  else
    (cd indexer && cargo test --lib route_exact_output -- --test-threads=1) &&
    (cd indexer && cargo test --test api_route_exact_output -- --test-threads=1)
  fi
}
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/pages/SwapPage.queryParams.test.tsx src/utils/cw20RouteSolveQuote.test.ts \
    src/services/terraclassic/router.reverse.test.ts src/services/indexer/__tests__/client.test.ts
}
docs() { python3 scripts/qa/check-issue-1389-docs.py; }
printf 'Revision: '; git rev-parse HEAD
date -u '+UTC: %Y-%m-%dT%H:%M:%SZ'
for pass in first retest; do
  for group in contracts indexer frontend; do
    reason_var="VERIFY_ISSUE_1389_SKIP_${group^^}_REASON"
    if [[ -n "${!reason_var:-}" ]]; then skip "$pass $group" "${!reason_var}"
    else run "$pass $group" "$group"; fi
  done
  run "$pass docs" docs
done
if [[ -n "${VERIFY_ISSUE_1389_CHAIN_COMMAND:-}" ]]; then
  run 'isolated LocalTerra gas' bash -c "$VERIFY_ISSUE_1389_CHAIN_COMMAND"
else
  skip 'isolated LocalTerra gas' "${VERIFY_ISSUE_1389_SKIP_CHAIN_REASON:-no gas command supplied; follow docs/qa/issue-1389/README.md}"
fi
printf '\n%s\n' "${results[@]}"
printf 'PASS=%s FAIL=%s SKIP=%s\n' "$passed" "$failed" "$skipped"
if (( failed )); then echo 'Overall FAIL: executed checks failed.'; exit 1; fi
if (( skipped )); then echo 'Overall PASS with explicit skips; skipped checks remain unverified.'
else echo 'Overall PASS'; fi
