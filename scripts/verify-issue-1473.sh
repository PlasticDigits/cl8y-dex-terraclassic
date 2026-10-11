#!/usr/bin/env bash
# Repeat a stable source patch; unavailable checks never become completed acceptance.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
PASS=0 FAIL=0 SKIP=0
declare -a RESULTS=()
run() {
  local label="$1"; shift
  printf '\n[%s]\n' "$label"
  if "$@"; then PASS=$((PASS+1)); RESULTS+=("PASS $label")
  else FAIL=$((FAIL+1)); RESULTS+=("FAIL $label"); fi
}
selected() {
  local label="$1" group="$2"; shift 2
  local reason_var="VERIFY_ISSUE_1473_SKIP_${group}_REASON"
  if [[ -n "${!reason_var:-}" ]]; then
    SKIP=$((SKIP+1)); RESULTS+=("SKIP $label: ${!reason_var}")
  else run "$label" "$@"; fi
}
fingerprint() {
  python3 - <<'PY'
import hashlib
from pathlib import Path
files=[]
for root in ['indexer/src','indexer/tests','frontend-dapp/src']:
    files.extend(p for p in Path(root).rglob('*') if p.is_file())
files.extend(Path(p) for p in ['docs/qa/issue-1473/matrix.json','scripts/qa/probe-issue-1473.py','scripts/qa/test-probe-issue-1473.py','scripts/qa/browser-trading-quotes-1473.cjs','scripts/qa/browser-live-trading-quotes-1473.cjs','scripts/qa/test-browser-live-trading-quotes-1473.cjs'])
h=hashlib.sha256()
for p in sorted(files):
    h.update(str(p).encode()+b'\0'+p.read_bytes()+b'\0')
print(h.hexdigest())
PY
}
indexer() { cargo test --locked --manifest-path indexer/Cargo.toml --lib -- --test-threads=1; }
socket_tests() {
  if [[ -f indexer/.env.qa ]]; then
    set -a
    source indexer/.env.qa
    set +a
  fi
  [[ -n "${TEST_DATABASE_ADMIN_URL:-${TEST_DATABASE_URL:-}}" ]] || {
    echo 'Dedicated disposable QA capability unavailable; provision .env.qa or record an explicit DB skip reason.' >&2
    return 1
  }
  local failed=0 suite
  for suite in route_local_quotes_1452 api_route_solve api_route_solve_db_hybrid api_route_exact_output api_route_tax_funding1381; do
    cargo test --locked --manifest-path indexer/Cargo.toml --test "$suite" -- --test-threads=1 || failed=1
  done
  return "$failed"
}
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/utils/routeQuoteErrors.test.ts src/utils/localRouteQuote.test.ts \
    src/hooks/useQuoteErrorRecovery.test.tsx src/hooks/useQuotePricingDataPending.test.tsx \
    src/utils/localQuoteFreshness.test.tsx src/components/QuoteDataLoadingProgress.test.tsx \
    src/utils/cw20RouteSolveQuote.test.ts src/utils/quoteDebounce.test.ts \
    src/utils/localWrapperOnlyQuote.test.ts src/hooks/useSubmitAlignedSimQuote.test.ts \
    src/components/swap/__tests__/ExpertModeModal.test.tsx \
    src/components/swap/ExpertModeModal.test.tsx \
    src/components/swap/__tests__/SwapPreSubmitSummary.test.tsx \
    src/services/terraclassic/__tests__/swapAutoGasFeeHint.test.tsx \
    src/utils/__tests__/indexerErrors.test.ts src/utils/__tests__/pairCatalogRank.test.ts \
    src/utils/__tests__/pairCatalogRank.issue562.test.ts \
    src/services/indexer/__tests__/client.test.ts \
    src/pages/SwapPage.queryParams.test.tsx src/pages/SwapPage.test.tsx \
    src/pages/TradePage.test.tsx src/utils/__tests__/tradeQueryResolve.test.ts \
    src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.slippagePresets.test.tsx
}
browser() { bash scripts/with-node.sh -- node scripts/qa/browser-trading-quotes-1473.cjs; }
types() { bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/tsc -b --pretty false; }
docs() {
  python3 - <<'PY'
from pathlib import Path
required={
 'AGENTS.md':['AGENTS_TRADING_QUOTE_OUTCOMES.md','verify-issue-1473'],
 'skills/AGENTS_TRADING_QUOTE_OUTCOMES.md':[*(f'TQ1473-{i}' for i in range(1,11)),'issue-1473/README.md'],
 'docs/qa/issue-1473/WARMUP.md':['expires_at_ms','indeterminate','SKIP','TQ1473'],
 'docs/qa/issue-1473/README.md':[*(f'AC{i}' for i in range(1,10)),'PASS WITH SKIPS','Executed failures remain FAIL'],
 'docs/route-solver.md':['#1473','indicative_amount_out','accounting_unavailable'],
 'docs/frontend.md':['#1473','native USTC','AGENTS_TRADING_QUOTE_OUTCOMES.md'],
 'docs/indexer-invariants.md':['#1473','AGENTS_TRADING_QUOTE_OUTCOMES.md'],
 'Makefile':['scripts/verify-issue-1473.sh'],
}
for filename,terms in required.items():
    body=Path(filename).read_text()
    for term in terms:
        assert term in body, f'{filename}: missing {term}'
print('Trading reason/default/invariant/agent checklist crosslinks PASS')
PY
}
printf 'Issue #1473 verification\nRevision: %s\nUTC start: %s\n' "$(git rev-parse HEAD)" "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
SOURCE_FINGERPRINT="$(fingerprint)"
printf 'Source fingerprint: %s\n' "$SOURCE_FINGERPRINT"
for attempt in first retest; do
  run "$attempt shared corpus and actual socket safety" python3 scripts/qa/test-probe-issue-1473.py
  run "$attempt live helper DOM and verdict regressions" bash scripts/with-node.sh -- node --test scripts/qa/test-browser-live-trading-quotes-1473.cjs
  selected "$attempt indexer integer/rejection/zero-RPC/security regressions" RUST indexer
  selected "$attempt disposable socket matrix and mode regressions" DB socket_tests
  selected "$attempt frontend intent/default/signing regressions" FRONTEND frontend
done
# The browser driver performs first/retest for every case in the same built artifact.
selected 'first/retest desktop/phone controlled browser matrix' BROWSER browser
selected 'frontend types' TYPES types
selected 'Rust formatting' FORMAT cargo fmt --manifest-path indexer/Cargo.toml -- --check
run 'verifier shell syntax' bash -n scripts/verify-issue-1473.sh
run 'invariants and checklist crosslinks' docs
run 'diff whitespace' git diff --check
if [[ "$(fingerprint)" == "$SOURCE_FINGERPRINT" ]]; then
  PASS=$((PASS+1)); RESULTS+=("PASS stable source across first/retest")
else
  FAIL=$((FAIL+1)); RESULTS+=("FAIL source changed; repeat verification on final stable source")
fi
SKIP=$((SKIP+2))
RESULTS+=("SKIP deployed frontend/backend provenance and sustained full matrix: inspect separate exact-revision deployment receipts")
RESULTS+=("SKIP physical devices and funded execution: inspect separate owned LocalTerra/device receipts")
printf '\n%s\n' "${RESULTS[@]}"
printf 'UTC end: %s\nPASS=%s FAIL=%s SKIP=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$PASS" "$FAIL" "$SKIP"
(( FAIL == 0 )) || { echo 'Overall FAIL: executed checks failed.'; exit 1; }
if (( SKIP )); then echo 'Overall PASS WITH SKIPS: tested source scope; skipped acceptance criteria remain unchecked.'
else echo 'Overall PASS: tested source scope.'; fi
