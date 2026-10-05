#!/usr/bin/env bash
# Synthetic oracle abuse + isolated Postgres/API/adapter + financial guards, twice.
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
  (cd indexer && cargo test --lib oracle -- --test-threads=1) &&
  (cd indexer && cargo test --lib hub_ -- --test-threads=1) &&
  (cd indexer && cargo test --lib economic_usd -- --test-threads=1) &&
  (cd indexer && cargo test --lib protocol_tvl -- --test-threads=1) &&
  (cd indexer && cargo test --test api_oracle --test api_hub_prices \
    --test indexer_protocol_liquidity --test indexer_protocol_liquidity_series \
    --test indexer_defillama --test volume_usd_catalog --test swap_price_human_usd \
    --test candle_human_usd --test candle_usd_mark -- --test-threads=1)
}
scripts() {
  python3 scripts/qa/test-oracle-safety.py &&
  python3 scripts/qa/test-oracle-lp-consumers.py &&
  python3 scripts/qa/test-custc-ust1-buyback-failures.py &&
  bash scripts/qa/test-ust1-clunc-buyback.sh &&
  bash scripts/qa/test-oracle-rebalance.sh &&
  bash scripts/qa/test-clunc-custc-lp.sh &&
  bash scripts/qa/test-clunc-usdt-lp.sh
}
for attempt in first retest; do
  for group in indexer scripts; do
    reason_var="VERIFY_ISSUE_1391_SKIP_${group^^}_REASON"
    if [[ -n "${!reason_var:-}" ]]; then skip "$attempt $group" "${!reason_var}"
    else run "$attempt $group" "$group"; fi
  done
done
run "patch whitespace" git diff --check
run "documentation links" python3 scripts/qa/check-issue-1391-docs.py
if [[ -n "${VERIFY_ISSUE_1391_CHAIN_COMMAND:-}" ]]; then
  run "isolated LocalTerra controls" bash -c "$VERIFY_ISSUE_1391_CHAIN_COMMAND"
else
  skip "isolated LocalTerra manipulation/LP controls" "${VERIFY_ISSUE_1391_SKIP_CHAIN_REASON:-no isolated funded fixture command supplied; probe/provision and record scope}"
fi
skip "production deployment acceptance" "${VERIFY_ISSUE_1391_SKIP_LIVE_REASON:-branch not deployed; implementation verification does not certify production}"
printf '\n%s\n' "${RESULTS[@]}"
printf '\nPASS=%s FAIL=%s SKIP=%s\n' "$PASS" "$FAIL" "$SKIP"
if (( FAIL )); then echo 'Overall FAIL: executed checks failed.'; exit 1; fi
if (( SKIP )); then echo 'Overall PASS with explicit skips; skipped criteria remain unverified.'
else echo 'Overall PASS: all checks passed.'; fi
