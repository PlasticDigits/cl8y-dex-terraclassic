#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
ISSUE=1500
source scripts/qa/verify-followup-common.sh
rust() {
  cargo test --locked --manifest-path indexer/Cargo.toml --lib issue1500 -- --test-threads=1
  cargo test --locked --manifest-path indexer/Cargo.toml --lib manual_split_post_prices_exact_controls_and_binds_cache_identity -- --test-threads=1
}
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/vitest run \
    src/utils/routeOrderMetrics.test.ts src/utils/routeEconomicExplanation.test.ts \
    src/utils/swapRouteSlippage.test.ts src/utils/cw20RouteSolveQuote.test.ts \
    src/components/swap/SwapImpactAside.test.tsx \
    src/components/swap/RouteOrderCostDetails.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.slippagePresets.test.tsx
}
documentation() {
  python3 - <<'PY'
from pathlib import Path
checks={
 'skills/AGENTS_ORDER_SLIPPAGE.md':[*(f'OS1500-{i}' for i in range(1,11)), 'issue-1500/README.md'],
 'docs/qa/issue-1500/README.md':[*(f'AC{i}' for i in range(1,7)), 'PASS WITH SKIPS', 'Executed failures remain FAIL'],
 'docs/route-solver.md':['selected_route_fee_free_v1', 'baseline - fee_free_output', 'Fair-reference deviation', '500 rows'],
 'docs/frontend.md':['OS1500'], 'docs/indexer-invariants.md':['OS1500'],
 'skills/AGENTS_ROUTE_COST_ATTRIBUTION.md':['OS1500'],
}
for name,terms in checks.items():
    body=Path(name).read_text()
    for term in terms: assert term in body, f'{name}: missing {term}'
print('Order metric contract and crosslinks PASS')
PY
}
start
for attempt in first retest; do
  selected "$attempt independent pool/book arithmetic" RUST rust
  selected "$attempt diagnostic forgery and original risk controls" FRONTEND frontend
done
selected 'frontend types' TYPES bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/tsc -b --pretty false
selected 'Rust formatting' FORMAT cargo fmt --manifest-path indexer/Cargo.toml -- --check
run 'verifier syntax' bash -n scripts/qa/verify-issue-1500.sh
run 'metric invariants, API and checklist crosslinks' documentation
skip 'Funded settlement and physical signing require separate actual receipts'
skip 'Attested deployed frontend/backend/contracts and sustained browser timing require separate actual receipts'
finish
