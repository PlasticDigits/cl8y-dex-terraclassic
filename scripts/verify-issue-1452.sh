#!/usr/bin/env bash
# Local-source acceptance; unavailable external checks stay explicit skips.
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
run_or_skip() {
  local label="$1" group="$2"; shift 2
  local reason_var="VERIFY_ISSUE_1452_SKIP_${group}_REASON"
  if [[ -n "${!reason_var:-}" ]]; then
    SKIP=$((SKIP+1)); RESULTS+=("SKIP $label: ${!reason_var}")
  else run "$label" "$@"; fi
}
source_fingerprint() {
  python3 - <<'PYTHON'
import hashlib
from pathlib import Path
files=[]
for root in ['indexer/src','indexer/tests','frontend-dapp/src']:
    files.extend(p for p in Path(root).rglob('*') if p.is_file())
files.extend(Path(p) for p in ['indexer/Cargo.toml','indexer/Cargo.lock','frontend-dapp/package.json','frontend-dapp/package-lock.json','scripts/qa/browser-local-quotes-1452.cjs'])
h=hashlib.sha256()
for p in sorted(files):
    h.update(str(p).encode()+b'\0'+p.read_bytes()+b'\0')
print(h.hexdigest())
PYTHON
}
indexer() {
  cargo test --locked --manifest-path indexer/Cargo.toml --lib -- --test-threads=1 || return 1
  cargo test --locked --manifest-path indexer/Cargo.toml --lib \
    lcd::pair_discovery_state::tests::pair_discovery_cache_benchmark -- --ignored --nocapture --test-threads=1
}
socket_tests() {
  # The suite's shared helper refuses runtime credentials and creates its own DB.
  if [[ -f indexer/.env.qa ]]; then
    set -a
    # shellcheck disable=SC1091
    source indexer/.env.qa
    set +a
  fi
  if [[ -z "${TEST_DATABASE_ADMIN_URL:-${TEST_DATABASE_URL:-}}" ]]; then
    echo 'Missing isolated QA credentials; provision indexer/.env.qa or use an explicit DB skip reason.' >&2
    return 1
  fi
  cargo test --locked --manifest-path indexer/Cargo.toml --test route_local_quotes_1452 --test api_route_solve_db_hybrid --test api_route_solve --test api_route_exact_output --test api_route_tax_funding1381 --test api_health -- --test-threads=1 --nocapture
}
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/utils/localRouteQuote.test.ts src/utils/cw20RouteSolveQuote.test.ts src/utils/localWrapperOnlyQuote.test.ts \
    src/utils/quoteDebounce.test.ts src/utils/swapPayAcquireGuidance.test.ts \
    src/utils/directHybridQuote.test.ts src/utils/swapReceiveQuote.test.ts \
    src/hooks/useSubmitAlignedSimQuote.test.ts \
    src/utils/slippageConstraint.test.ts \
    src/services/terraclassic/__tests__/transactions.test.ts \
    src/services/terraclassic/__tests__/terraBroadcast.autoGas.test.ts \
    src/hooks/__tests__/useLimitOrderMakerFeeRates.test.tsx \
    src/components/trade/__tests__/TradeOrderTicket.invert.test.tsx \
    src/pages/SwapPage.test.tsx \
    src/pages/SwapPage.acquireGuidance.test.tsx src/pages/SwapPage.assetDecimals.test.tsx \
    src/pages/SwapPage.extraDebitSell.test.tsx src/pages/SwapPage.feeDiscountRegistryBanner.test.tsx \
    src/pages/SwapPage.queryParams.test.tsx src/pages/TradePage.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.acquireGuidance.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.extraDebitSell.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.slippagePresets.test.tsx \
    src/services/terraclassic/__tests__/swapAutoGasFeeHint.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx
}
browser() { bash scripts/with-node.sh -- node scripts/qa/browser-local-quotes-1452.cjs; }
frontend_types() { bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/tsc -b --pretty false; }
docs() {
  python3 - <<'PY'
from pathlib import Path
required = {
    'AGENTS.md': ['AGENTS_LOCAL_SWAP_QUOTES.md', 'verify-issue-1452'],
    'docs/route-solver.md': ['Local quote contract (#1452)', 'Historical request solver reference', 'qa/issue-1452/RESULTS.md'],
    'docs/frontend.md': ['#1452', 'AGENTS_LOCAL_SWAP_QUOTES.md'],
    'docs/swap-max-spread-ux.md': ['#1452', 'AGENTS_LOCAL_SWAP_QUOTES.md'],
    'docs/indexer-invariants.md': ['#1452', 'AGENTS_LOCAL_SWAP_QUOTES.md'],
    'skills/AGENTS_LOCAL_SWAP_QUOTES.md': [f'LQ1452-{i}' for i in range(1,11)],
    'docs/qa/issue-1452/README.md': [*(f'AC{i}' for i in range(1,11)), 'PASS WITH SKIPS', 'FAIL'],
    'docs/qa/issue-1452/RESULTS.md': ['Supported', 'SKIP'],
}
for filename, terms in required.items():
    body=Path(filename).read_text()
    for term in terms:
        assert term in body, f'{filename}: missing {term}'
for module in ['local_quote', 'local_hook_pricing', 'local_wrapper_quote']:
    assert f'mod {module};' in Path('indexer/src/api/mod.rs').read_text(), module
    assert '#[test]' in Path(f'indexer/src/api/{module}.rs').read_text() or '#[tokio::test]' in Path(f'indexer/src/api/{module}.rs').read_text(), f'{module}: missing tests'
assert '#[tokio::test]' in Path('indexer/tests/route_local_quotes_1452.rs').read_text()
print('Current invariant/checklist/test crosslinks: PASS')
PY
}
printf 'Issue #1452 verification at %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
printf 'Revision: %s (including working tree)\n' "$(git rev-parse HEAD)"
SOURCE_FINGERPRINT="$(source_fingerprint)"
printf 'Source fingerprint: %s\n' "$SOURCE_FINGERPRINT"
for attempt in first retest; do
  run_or_skip "$attempt indexer library" RUST indexer
  run_or_skip "$attempt isolated socket selection/cache/network/performance" DB socket_tests
  run_or_skip "$attempt frontend quote/refresh/signing" FRONTEND frontend
  run_or_skip "$attempt production-build controlled browser" BROWSER browser
done
run_or_skip 'frontend types' TYPES frontend_types
run_or_skip 'indexer formatting' FORMAT cargo fmt --manifest-path indexer/Cargo.toml -- --check
run 'shell syntax' bash -n scripts/verify-issue-1452.sh
run 'diff whitespace' git diff --check
run 'documentation and invariant crosslinks' docs
if [[ "$(source_fingerprint)" == "$SOURCE_FINGERPRINT" ]]; then
  PASS=$((PASS+1)); RESULTS+=("PASS unchanged source during verification/retest")
else
  FAIL=$((FAIL+1)); RESULTS+=("FAIL source changed during verification; rerun on stable source")
fi
printf '\n%s\n' "${RESULTS[@]}"
printf '\nResults: %s PASS, %s FAIL, %s SKIP\n' "$PASS" "$FAIL" "$SKIP"
(( FAIL == 0 )) || { echo 'Overall: FAIL'; exit 1; }
if (( SKIP > 0 )); then echo 'Overall: PASS WITH SKIPS (local implementation only)'
else echo 'Overall: PASS (local implementation only; deployed/browser acceptance separate)'; fi
