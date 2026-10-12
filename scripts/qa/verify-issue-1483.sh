#!/usr/bin/env bash
# Focused stable-source attribution, amount-cache ranking and actual review checks.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
ISSUE=1483
source scripts/qa/verify-followup-common.sh
rust() {
  cargo test --locked --manifest-path indexer/Cargo.toml --lib api::local_quote::attribution -- --test-threads=1 || return 1
  cargo test --locked --manifest-path indexer/Cargo.toml --lib issue1483 -- --test-threads=1
}
sockets() {
  if [[ -f indexer/.env.qa ]]; then
    set -a
    source indexer/.env.qa
    set +a
  fi
  [[ -n "${TEST_DATABASE_ADMIN_URL:-${TEST_DATABASE_URL:-}}" ]] || {
    echo 'Dedicated disposable QA capability unavailable; provision it or record explicit DB skip.' >&2
    return 1
  }
  cargo test --locked --manifest-path indexer/Cargo.toml --test route_local_quotes_1452 issue1483 -- --test-threads=1 --nocapture
}
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/utils/routeEconomicExplanation.test.ts src/utils/swapRouteSlippage.test.ts \
    src/components/swap/SwapImpactAside.test.tsx src/utils/tradingNotice.test.ts \
    src/utils/__tests__/humanizeTerraTxError.test.ts \
    src/services/terraclassic/__tests__/swapAutoGasFeeHint.test.tsx \
    src/components/swap/__tests__/SwapPreSubmitSummary.test.tsx \
    src/utils/localRouteQuote.test.ts src/utils/localWrapperOnlyQuote.test.ts \
    src/pages/SwapPage.test.tsx src/pages/TradePage.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.slippagePresets.test.tsx
}
documentation() {
  python3 - <<'PY'
from pathlib import Path
checks={
 'AGENTS.md':['AGENTS_ROUTE_COST_ATTRIBUTION.md','verify-issue-1483'],
 'Makefile':['scripts/qa/verify-issue-1483.sh'],
 'skills/AGENTS_ROUTE_COST_ATTRIBUTION.md':[*(f'RC1483-{i}' for i in range(1,11)),'issue-1483/README.md'],
 'docs/qa/issue-1483/README.md':[*(f'AC{i}' for i in range(1,8)),'PASS WITH SKIPS','Executed failures remain FAIL','RESULTS.md'],
 'docs/route-solver.md':['economic_attribution','spot_bridge_v1','#1483'],
 'docs/frontend.md':['#1483','AGENTS_ROUTE_COST_ATTRIBUTION.md'],
 'docs/indexer-invariants.md':['#1483','AGENTS_ROUTE_COST_ATTRIBUTION.md'],
}
for name, terms in checks.items():
    body=Path(name).read_text()
    for term in terms: assert term in body, f'{name}: missing {term}'
print('Attribution contract, acceptance, verifier and agent crosslinks PASS')
PY
}
start
for attempt in first retest; do
  run "$attempt public probe privacy, cap and attribution boundary" python3 scripts/qa/test-probe-route-cost-attribution-1483.py
  run "$attempt private wallet helper least-scope and no-disclosure boundary" python3 scripts/qa/test-prepare-wallet-attribution-1483.py
  run "$attempt deployed explanation source, clock and public evidence boundary" bash scripts/with-node.sh -- node --test scripts/qa/test-browser-route-attribution-1483.cjs
  selected "$attempt signed raw attribution and unavailable boundaries" RUST rust
  selected "$attempt exact-cohort amount-cache ranking, wrapper and zero-RPC sockets" DB sockets
  selected "$attempt actual Swap/Trade explanation and execution guards" FRONTEND frontend
done
selected 'frontend types' TYPES bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/tsc -b --pretty false
selected 'Rust formatting' FORMAT cargo fmt --manifest-path indexer/Cargo.toml -- --check
run 'verifier syntax' bash -n scripts/qa/verify-issue-1483.sh
run 'owned funded driver syntax' python3 -c "import ast,pathlib; ast.parse(pathlib.Path('scripts/qa/localterra-route-attribution-1483.py').read_text()); ast.parse(pathlib.Path('scripts/qa/localterra-quote-protection-1473.py').read_text())"
run 'API, invariants and checklist crosslinks' documentation
skip 'Complete funded native/wrapped/tax/hook/book/device scope requires actual separate receipts'
skip 'Independent deployed backend/frontend/contracts, sustained browser timing and monitoring require actual separate receipts'
finish
