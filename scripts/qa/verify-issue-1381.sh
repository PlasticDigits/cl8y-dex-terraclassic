#!/usr/bin/env bash
# Versioned route-funded tax checks, retest, and truthful optional-check accounting.
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
  local target="${CONTRACTS_CARGO_TARGET_DIR:-$PWD/smartcontracts/target}"
  env CARGO_TARGET_DIR="$target" cargo test --manifest-path smartcontracts/Cargo.toml --workspace -- --test-threads=1 || return 1
  env CARGO_TARGET_DIR="$target" cargo test --manifest-path smartcontracts/Cargo.toml -p dex-common route_tax -- --test-threads=1 || return 1
}
indexer() {
  local target="${INDEXER_CARGO_TARGET_DIR:-$PWD/indexer/target}"
  # Keep indexer memory/write volume independent of contract compiler profiles.
  local debug="${INDEXER_CARGO_TEST_DEBUG:-${CARGO_PROFILE_TEST_DEBUG:-2}}"
  local incremental="${INDEXER_CARGO_INCREMENTAL:-${CARGO_INCREMENTAL:-1}}"
  env CARGO_PROFILE_DEV_DEBUG="$debug" CARGO_PROFILE_TEST_DEBUG="$debug" \
    CARGO_INCREMENTAL="$incremental" CARGO_TARGET_DIR="$target" cargo test --manifest-path indexer/Cargo.toml --lib -- --test-threads=1 || return 1
  (cd indexer && env CARGO_PROFILE_DEV_DEBUG="$debug" CARGO_PROFILE_TEST_DEBUG="$debug" \
    CARGO_INCREMENTAL="$incremental" CARGO_TARGET_DIR="$target" cargo test \
    --test api_route_solve --test api_route_solve_db_hybrid \
    --test api_route_exact_output --test api_route_tax_funding1381 \
    -- --test-threads=1) || return 1
}
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/hooks/__tests__/useCommunityTaxSellBps.test.tsx \
    src/services/terraclassic/routeTaxFunding.test.ts \
    src/services/terraclassic/routeTaxFundingBudget.test.ts \
    src/pages/SwapPage.extraDebitSell.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.extraDebitSell.test.tsx \
    src/components/payments/PayWithAnyToken.test.tsx \
    src/utils/payInvoice.test.ts src/utils/cw20RouteSolveQuote.test.ts \
    src/utils/directHybridQuote.test.ts \
    src/services/terraclassic/__tests__/terraGas.mixedHybrid.test.ts \
    src/services/terraclassic/__tests__/terraGas.issue1328.test.ts \
    src/services/terraclassic/__tests__/terraGas.retailShapes.test.ts \
    src/services/terraclassic/__tests__/terraGas.greedy.test.ts \
    src/services/terraclassic/__tests__/swapNetworkFee.test.ts \
    src/services/terraclassic/__tests__/swapAutoGas.test.ts \
    src/services/terraclassic/__tests__/swapAutoGasFeeHint.test.tsx \
    src/utils/__tests__/maxSpendableAmount.test.ts \
    src/utils/taxPreviewMaxSpend.test.ts src/utils/__tests__/humanizeTerraTxError.test.ts
}
docs() { python3 scripts/qa/check-issue-1381-docs.py; }
chain_execution() {
  local evidence="${VERIFY_ISSUE_1381_CHAIN_EVIDENCE:-/tmp/dex-1381-localterra.json}"
  local container="${VERIFY_ISSUE_1381_CONTAINER:-cl8y-dex-terraclassic-localterra-1}"
  local args=()
  if [[ -n "${VERIFY_ISSUE_1381_LEGACY_ARTIFACTS:-}" ]]; then
    args+=(--legacy-artifacts "$VERIFY_ISSUE_1381_LEGACY_ARTIFACTS")
  fi
  python3 scripts/qa/localterra-route-tax-1381.py --isolated-owner "${args[@]}" \
    --container "$container" \
    --artifacts "${VERIFY_ISSUE_1381_ARTIFACTS:-smartcontracts/artifacts}" \
    --code-cache "${VERIFY_ISSUE_1381_CODE_CACHE:-/tmp/dex-1381-localterra-codes.json}" \
    --evidence-file "$evidence" || return 1
  args=()
  if [[ -n "${VERIFY_ISSUE_1381_WRAP_STACK_ENV:-}" ]]; then
    args+=(--stack-env "$VERIFY_ISSUE_1381_WRAP_STACK_ENV")
  fi
  python3 scripts/qa/localterra-route-tax-flows-1381.py "${args[@]}" \
    --container "$container" --fixture "$evidence" \
    --evidence-file "${VERIFY_ISSUE_1381_FLOW_EVIDENCE:-/tmp/dex-1381-localterra-flows.json}"
}
for pass in first retest; do
  for group in contracts indexer frontend; do
    reason_var="VERIFY_ISSUE_1381_SKIP_${group^^}_REASON"
    if [[ -n "${!reason_var:-}" ]]; then skip "$pass $group" "${!reason_var}"
    else run "$pass $group" "$group"; fi
  done
  run "$pass docs" docs
done
if [[ -n "${VERIFY_ISSUE_1381_SKIP_STATIC_REASON:-}" ]]; then
  skip "lint/types/build" "$VERIFY_ISSUE_1381_SKIP_STATIC_REASON"
else
  run "contract formatting" cargo fmt --manifest-path smartcontracts/Cargo.toml --all --check
  run "indexer changed-source formatting" python3 scripts/qa/check-issue-1381-format.py
  run "frontend lint" make lint-frontend
  # Synthetic public build-only WalletConnect ID; never disable production guards.
  run "frontend types and build" env VITE_DEV_MODE=false \
    VITE_WC_PROJECT_ID="${VITE_WC_PROJECT_ID:-13811381138113811381138113811381}" \
    bash scripts/with-node.sh --cwd frontend-dapp -- npm run build
fi
if [[ -n "${VERIFY_ISSUE_1381_SKIP_CHAIN_REASON:-}" ]]; then
  skip "LocalTerra execute" "$VERIFY_ISSUE_1381_SKIP_CHAIN_REASON"
else
  run "LocalTerra migrations, funding and atomic invoice/native flows" chain_execution
  if [[ -z "${VERIFY_ISSUE_1381_WRAP_STACK_ENV:-}" ]]; then
    skip "native wrap/unwrap" "No existing LocalTerra public wrap-stack env supplied; use VERIFY_ISSUE_1381_WRAP_STACK_ENV"
  fi
fi
if [[ -n "${VERIFY_ISSUE_1381_SKIP_LIVE_REASON:-}" ]]; then
  skip "deployed capability and execution" "$VERIFY_ISSUE_1381_SKIP_LIVE_REASON"
elif [[ -n "${VERIFY_ISSUE_1381_LIVE_COMMAND:-}" ]]; then
  run "deployed capability and execution" bash -c "$VERIFY_ISSUE_1381_LIVE_COMMAND"
else
  skip "deployed capability and execution" "No authorized coordinated migration/execution command supplied; follow the runbook"
fi
printf '\n%s\n' "${RESULTS[@]}"
printf '\nPASS=%s FAIL=%s SKIP=%s\n' "$PASS" "$FAIL" "$SKIP"
if (( FAIL )); then echo 'Overall FAIL: executed checks failed.'; exit 1; fi
if (( SKIP )); then echo 'Overall PASS with explicit skips; skipped criteria remain unverified.'
else echo 'Overall PASS: all checks passed.'; fi
