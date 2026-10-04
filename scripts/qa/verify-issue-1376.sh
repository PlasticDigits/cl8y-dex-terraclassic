#!/usr/bin/env bash
# #1376: bounded solver receive intent + pair inverse + finite errors.
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
contracts() { (cd smartcontracts && cargo test --workspace issue1376 -- --test-threads=1); }
indexer() {
  if [[ -f indexer/.env.issue1376 ]]; then
    # Integration bootstrap deliberately loads .env with override; use the isolated test DB.
    python3 - <<'PY'
from pathlib import Path
source = Path('indexer/.env.issue1376').read_text().splitlines()
value = next(line for line in source if line.startswith('TEST_DATABASE_URL='))
env = Path('indexer/.env')
lines = env.read_text().splitlines() if env.exists() else []
env.write_text('\n'.join([line for line in lines if not line.startswith('TEST_DATABASE_URL=')] + [value]) + '\n')
PY
  fi
  (cd indexer && cargo test --lib route_exact_output -- --test-threads=1)
  (cd indexer && cargo test --test api_route_exact_output -- --test-threads=1)
}
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/pages/SwapPage.queryParams.test.tsx src/utils/cw20RouteSolveQuote.test.ts \
    src/utils/swapReceiveQuote.test.ts src/utils/receiveQuoteDeadline.test.ts \
    src/services/terraclassic/router.reverse.test.ts src/services/terraclassic/router.test.ts \
    src/utils/communityTaxNetOut.test.ts src/utils/payInvoice.test.ts \
    src/utils/__tests__/swapQueryParams.test.ts src/utils/hybridHopOfferPartition.test.ts
}
docs() {
  python3 scripts/qa/check-issue-1376-docs.py
  python3 scripts/check_chrome_nesting.py
}
for pass in first retest; do
  for group in contracts indexer frontend; do
    reason_var="VERIFY_ISSUE_1376_SKIP_${group^^}_REASON"
    if [[ -n "${!reason_var:-}" ]]; then skip "$pass $group" "${!reason_var}"
    else run "$pass $group" "$group"; fi
  done
  run "$pass docs" docs
done
run "frontend lint" make lint-frontend
run "frontend types" bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/tsc -b --pretty false
if [[ -n "${VERIFY_ISSUE_1376_SKIP_BROWSER_REASON:-}" ]]; then
  skip browser "$VERIFY_ISSUE_1376_SKIP_BROWSER_REASON"
else
  run "browser receive lifecycle and quote errors (mocked LCD)" \
    env E2E_SWAP_RECEIVE_1256=1 PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-3026}" \
    bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test \
    --project=e2e-smoke --workers=2 e2e/swap-receive-1256.spec.ts
fi
if [[ "${VERIFY_ISSUE_1376_LIVE:-0}" == 1 ]]; then
  run "live migrated pair inventory and 10/100 probes" python3 scripts/qa/probe-issue-1376.py --expect-fixed
  run "fresh live browser 10/100 replay" node scripts/qa/browser-probe-issue-1376.cjs --expect-fixed
else
  skip "live deployment/browser replay" "${VERIFY_ISSUE_1376_SKIP_LIVE_REASON:-new versions have not been deployed; follow docs/qa/issue-1376/README.md}"
fi
if [[ -n "${VERIFY_ISSUE_1376_CHAIN_COMMAND:-}" ]]; then
  run "LocalTerra execution" bash -c "$VERIFY_ISSUE_1376_CHAIN_COMMAND"
else
  skip "LocalTerra wallet execution" "${VERIFY_ISSUE_1376_SKIP_CHAIN_REASON:-no execution command supplied; record provision attempt and run the QA checklist}"
fi
printf '\n%s\n' "${RESULTS[@]}"
printf '\nPASS=%s FAIL=%s SKIP=%s\n' "$PASS" "$FAIL" "$SKIP"
if (( FAIL )); then echo 'Overall FAIL: executed checks failed.'; exit 1; fi
if (( SKIP )); then echo 'Overall PASS with explicit skips; skipped checks remain unverified.'
else echo 'Overall PASS: all selected checks passed.'; fi
