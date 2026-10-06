#!/usr/bin/env bash
# #1390 runnable implementation checks, retest, and explicit unavailable scope.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
PASS=0 FAIL=0 SKIP=0
declare -a RESULTS=()
run() {
  local label="$1"; shift
  printf '\n[%s]\n' "$label"
  if "$@"; then PASS=$((PASS+1)); RESULTS+=("PASS $label")
  else FAIL=$((FAIL+1)); RESULTS+=("FAIL $label"); fi
}
skip() { SKIP=$((SKIP+1)); RESULTS+=("SKIP $1: $2"); }
contracts() {
  env CARGO_TARGET_DIR="${CONTRACTS_CARGO_TARGET_DIR:-$PWD/smartcontracts/target}" \
    cargo test --manifest-path smartcontracts/Cargo.toml --workspace issue1390 -- --test-threads=1
}
indexer() {
  local target="${INDEXER_CARGO_TARGET_DIR:-$PWD/indexer/target}"
  # Keep indexer memory/write volume independent of contract compiler profiles.
  local debug="${INDEXER_CARGO_TEST_DEBUG:-${CARGO_PROFILE_TEST_DEBUG:-2}}"
  local incremental="${INDEXER_CARGO_INCREMENTAL:-${CARGO_INCREMENTAL:-1}}"
  for filter in community_tax_rank route_exact_output router_preview_rejection; do
    env CARGO_PROFILE_DEV_DEBUG="$debug" CARGO_PROFILE_TEST_DEBUG="$debug" \
    CARGO_INCREMENTAL="$incremental" CARGO_TARGET_DIR="$target" cargo test --manifest-path indexer/Cargo.toml \
      --lib "$filter" -- --test-threads=1 || return 1
  done
  (cd indexer && env CARGO_PROFILE_DEV_DEBUG="$debug" CARGO_PROFILE_TEST_DEBUG="$debug" \
    CARGO_INCREMENTAL="$incremental" CARGO_TARGET_DIR="$target" cargo test \
    --test api_route_tax_funding1381 --test api_route_exact_output -- --test-threads=1)
}
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/services/terraclassic/routeTaxPath.test.ts \
    src/services/terraclassic/routeTaxFunding.test.ts \
    src/services/terraclassic/routeTaxFundingBudget.test.ts \
    src/services/terraclassic/__tests__/swapRoutePreflight.test.ts \
    src/services/terraclassic/router.test.ts src/services/terraclassic/router.reverse.test.ts \
    src/utils/cw20RouteSolveQuote.test.ts src/utils/directHybridQuote.test.ts \
    src/utils/payInvoice.test.ts src/components/payments/PayWithAnyToken.test.tsx \
    src/services/indexer/__tests__/client.test.ts src/utils/__tests__/indexerErrors.test.ts \
    src/pages/SwapPage.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx
}
docs() { python3 scripts/qa/check-issue-1390-docs.py; }
for pass in first retest; do
  for group in contracts indexer frontend; do
    reason_var="VERIFY_ISSUE_1390_SKIP_${group^^}_REASON"
    if [[ -n "${!reason_var:-}" ]]; then skip "$pass $group" "${!reason_var}"
    else run "$pass $group" "$group"; fi
  done
  run "$pass docs" docs
done
if [[ -n "${VERIFY_ISSUE_1390_SKIP_STATIC_REASON:-}" ]]; then
  skip "format/lint/build" "$VERIFY_ISSUE_1390_SKIP_STATIC_REASON"
else
  run "contracts format" cargo fmt --manifest-path smartcontracts/Cargo.toml --all --check
  run "indexer changed-source format" python3 scripts/qa/check-issue-1381-format.py
  run "frontend lint" make lint-frontend
  run "frontend types/build" env VITE_DEV_MODE=false \
    VITE_WC_PROJECT_ID="${VITE_WC_PROJECT_ID:-13901390139013901390139013901390}" \
    bash scripts/with-node.sh --cwd frontend-dapp -- npm run build
  run "chain helper syntax" python3 -c \
    'from pathlib import Path; p=Path("scripts/qa/localterra-route-guards-1390.py"); compile(p.read_text(), str(p), "exec")'
  run "verifier syntax" bash -n scripts/qa/verify-issue-1390.sh
fi
for group in chain live; do
  reason_var="VERIFY_ISSUE_1390_SKIP_${group^^}_REASON"
  command_var="VERIFY_ISSUE_1390_${group^^}_COMMAND"
  if [[ -n "${!reason_var:-}" ]]; then skip "$group execution" "${!reason_var}"
  elif [[ -n "${!command_var:-}" ]]; then run "$group execution" bash -c "${!command_var}"
  else skip "$group execution" "No execution command supplied; follow docs/qa/issue-1390/README.md"; fi
done
printf '\n%s\n' "${RESULTS[@]}"
printf '\nPASS=%s FAIL=%s SKIP=%s\n' "$PASS" "$FAIL" "$SKIP"
if (( FAIL )); then echo 'Overall FAIL: executed checks failed.'; exit 1; fi
if (( SKIP )); then echo 'Overall PASS with explicit skips; skipped criteria remain unverified.'
else echo 'Overall PASS: all selected checks passed.'; fi
