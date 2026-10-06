#!/usr/bin/env bash
# Synthetic endpoint trust regressions, retest and explicit unavailable-check scope.
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
indexer() {
  local target="${INDEXER_CARGO_TARGET_DIR:-$PWD/indexer/target}"
  env CARGO_TARGET_DIR="$target" cargo test --manifest-path indexer/Cargo.toml --lib -- --test-threads=1 || return 1
  env CARGO_TARGET_DIR="$target" cargo check --manifest-path indexer/Cargo.toml --tests || return 1
}
integration() {
  local target="${INDEXER_CARGO_TARGET_DIR:-$PWD/indexer/target}"
  (cd indexer && env CARGO_TARGET_DIR="$target" cargo test --test api_health \
    --test api_fee_discount_health --test api_limit_book_lcd_mock \
    --test book_snapshot_loop --test indexer_ingestion_hardening \
    --test api_route_tax_funding1381 -- --test-threads=1)
}
treasury() {
  python3 scripts/qa/test-chain-attestation.py || return 1
  bash scripts/qa/test-custc-ust1-buyback.sh || return 1
  bash scripts/qa/test-ust1-clunc-buyback.sh || return 1
  bash scripts/qa/test-oracle-rebalance.sh || return 1
  bash scripts/qa/test-clunc-custc-lp.sh || return 1
  bash scripts/qa/test-clunc-usdt-lp.sh || return 1
}
syntax() {
  local script
  for script in scripts/lib/chain-attestation.sh scripts/lib/terrad-host.sh \
    scripts/mint-swap-custc-ust1.sh scripts/mint-swap-burn-ust1-clunc.sh \
    scripts/rebalance-mint-ust1-lp.sh scripts/rebalance-oracle-mint-swap-burn.sh \
    scripts/rebalance-mint-clunc-custc-lp.sh scripts/mint-clunc-usdt-lp.sh \
    scripts/rotate-fee-treasury.sh scripts/multisig-2of3-host-tx.sh \
    scripts/deploy-dex-local.sh scripts/deploy-dex-mainnet-soft-launch.sh; do
    bash -n "$script" || return 1
  done
}
for attempt in first retest; do
  for group in indexer integration treasury; do
    reason_var="VERIFY_ISSUE_1392_SKIP_${group^^}_REASON"
    if [[ -n "${!reason_var:-}" ]]; then skip "$attempt $group" "${!reason_var}"
    else run "$attempt $group" "$group"; fi
  done
  run "$attempt docs" python3 scripts/qa/check-issue-1392-docs.py
done
run "shell syntax" syntax
run "diff whitespace" git diff --check
# These are read-only commands prepared by the verifying agent using public/isolated endpoints.
# Do not include signing, deploy or secrets in them.
for scope in local live; do
  command_var="VERIFY_ISSUE_1392_${scope^^}_COMMAND"
  reason_var="VERIFY_ISSUE_1392_SKIP_${scope^^}_REASON"
  if [[ -n "${!command_var:-}" ]]; then run "$scope read-only attestation" bash -c "${!command_var}"
  elif [[ -n "${!reason_var:-}" ]]; then skip "$scope read-only attestation" "${!reason_var}"
  else skip "$scope read-only attestation" "No read-only endpoint command supplied; attempt prerequisites and record scope in verification.md"; fi
done
printf '\n%s\n' "${RESULTS[@]}"
printf '\nPASS=%s FAIL=%s SKIP=%s\n' "$PASS" "$FAIL" "$SKIP"
if (( FAIL )); then echo 'Overall FAIL: executed checks failed.'; exit 1; fi
if (( SKIP )); then echo 'Overall PASS with explicit skips; skipped criteria remain unverified.'
else echo 'Overall PASS: all selected checks passed.'; fi
