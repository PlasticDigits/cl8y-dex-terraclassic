#!/usr/bin/env bash
# #1380: honest liquidity quality, terminal history and bounded reserve refresh.
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
  (cd indexer && cargo test --lib protocol_tvl -- --test-threads=1) &&
  (cd indexer && cargo test --lib book_snapshot -- --test-threads=1) &&
  (cd indexer && cargo test --test indexer_protocol_liquidity --test indexer_protocol_liquidity_series \
    --test api_overview --test book_snapshot_loop -- --test-threads=1)
}
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/pages/ProtocolPage.test.tsx src/utils/__tests__/protocolVolumeGrain.test.ts \
    src/utils/__tests__/formatProtocolStats.test.ts src/utils/__tests__/protocolLiquidityQuality.test.ts
}
for attempt in first retest; do
  for group in indexer frontend; do
    reason_var="VERIFY_ISSUE_1380_SKIP_${group^^}_REASON"
    if [[ -n "${!reason_var:-}" ]]; then skip "$attempt $group" "${!reason_var}"
    else run "$attempt $group" "$group"; fi
  done
done
run "types" bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/tsc -b --pretty false
run "frontend lint" make lint-frontend
run "documentation links" python3 -c 'from pathlib import Path; paths=["docs/qa/issue-1380/README.md","docs/qa/issue-1380/verification.md","skills/AGENTS_PROTOCOL_LIQUIDITY_QUALITY.md"]; assert all(Path(p).is_file() for p in paths); assert "Q1380-9" in Path("docs/indexer-invariants.md").read_text()'
if [[ -n "${VERIFY_ISSUE_1380_SKIP_BROWSER_REASON:-}" ]]; then skip browser "$VERIFY_ISSUE_1380_SKIP_BROWSER_REASON"
else
  run "browser desktop/mobile quality" env PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30380 \
    PLAYWRIGHT_BASE_URL=http://127.0.0.1:30380 bash scripts/with-node.sh --cwd frontend-dapp -- \
    ./node_modules/.bin/playwright test --project=e2e-smoke --workers=1 --output=/tmp/issue1380-quality-verifier-artifacts e2e/protocol-liquidity-quality.spec.ts
fi
if [[ -n "${VERIFY_ISSUE_1380_SKIP_REGRESSIONS_REASON:-}" ]]; then skip regressions "$VERIFY_ISSUE_1380_SKIP_REGRESSIONS_REASON"
else
  for issue in 569 689 655 664 1263; do
    # Parent covers the selected regression union. Avoid recursively repeating it.
    run "regression $issue" env PLAYWRIGHT_OUTPUT_DIR="/tmp/issue1380-regression-$issue-artifacts" PLAYWRIGHT_WORKERS="${PLAYWRIGHT_WORKERS:-2}" VERIFY_ISSUE_569_SKIP_RELATED=1 VERIFY_ISSUE_689_SKIP_RELATED=1 \
      VERIFY_ISSUE_655_SKIP_RELATED=1 VERIFY_ISSUE_1263_SKIP_RELATED=1 make "verify-issue-$issue"
  done
fi
if [[ -n "${VERIFY_ISSUE_1380_CHAIN_COMMAND:-}" ]]; then run "LocalTerra controls" bash -c "$VERIFY_ISSUE_1380_CHAIN_COMMAND"
else skip "LocalTerra outage/LP controls" "${VERIFY_ISSUE_1380_SKIP_CHAIN_REASON:-no isolated funded withdrawal/outage fixture supplied; attempt provisioning and document remaining scope}"; fi
if [[ "${VERIFY_ISSUE_1380_LIVE:-0}" == 1 ]]; then
  run "live public evidence" python3 scripts/qa/probe-issue-1380.py --expect-quality --output /tmp/issue-1380-public.json
else skip "deployed revision/history confirmation" "${VERIFY_ISSUE_1380_SKIP_LIVE_REASON:-PR has not been deployed; public reads cannot prove new implementation}"; fi
printf '\n%s\n' "${RESULTS[@]}"
printf '\nPASS=%s FAIL=%s SKIP=%s\n' "$PASS" "$FAIL" "$SKIP"
if (( FAIL )); then echo 'Overall FAIL: executed checks failed.'; exit 1; fi
if (( SKIP )); then echo 'Overall PASS with explicit skips; skipped checks remain unverified.'
else echo 'Overall PASS: all selected checks passed.'; fi
