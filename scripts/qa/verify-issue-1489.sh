#!/usr/bin/env bash
# Stable exact-intent discovery, producer revocation and adoption regressions.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
ISSUE=1489
source scripts/qa/verify-followup-common.sh
fingerprint() {
  python3 - <<'PY'
import hashlib
from pathlib import Path
files = set()
for root in ['indexer/src', 'indexer/tests', 'frontend-dapp/src', 'smartcontracts/contracts', 'smartcontracts/packages']:
    files.update(p for p in Path(root).rglob('*') if p.is_file())
files.update(Path('scripts/qa').glob('*1489*'))
files.update(Path(p) for p in [
    'indexer/Cargo.toml', 'indexer/Cargo.lock', 'frontend-dapp/package.json', 'frontend-dapp/package-lock.json',
    'scripts/qa/verify-followup-common.sh', 'scripts/qa/browser-route-attribution-1483.cjs',
    'scripts/qa/browser-case-lifecycle.cjs', 'scripts/qa/test-browser-case-lifecycle.cjs',
    'scripts/qa/browser-live-trading-quotes-1473.cjs', 'scripts/qa/probe-route-cost-attribution-1483.py',
    'scripts/qa/prepare-wallet-attribution-1483.py', 'scripts/qa/test-prepare-wallet-attribution-1483.py',
    'scripts/qa/localterra-route-attribution-1483.py', 'scripts/qa/attest-trading-quotes-1473.py',
])
h = hashlib.sha256()
for p in sorted(files):
    if p.is_file(): h.update(str(p).encode() + b'\0' + p.read_bytes() + b'\0')
print(h.hexdigest())
PY
}
rust() {
  cargo test --locked --manifest-path indexer/Cargo.toml --lib issue1489 -- --test-threads=1
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
  cargo test --locked --manifest-path indexer/Cargo.toml --test route_local_quotes_1452 issue1489 -- --test-threads=1 --nocapture
}
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/utils/routeSolveProgress.test.ts src/hooks/useRouteSolveProgress.test.tsx \
    src/hooks/useRouteQuotePromotion.test.tsx \
    src/utils/nativeWrapRouteSolve.test.ts src/utils/routeEconomicExplanation.test.ts \
    src/utils/localRouteQuote.test.ts src/utils/swapRouteSlippage.test.ts \
    src/services/indexer/__tests__/client.test.ts \
    src/pages/SwapPage.test.tsx src/pages/TradePage.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.slippagePresets.test.tsx
}
documentation() {
  python3 - <<'PY'
from pathlib import Path
checks = {
 'AGENTS.md': ['AGENTS_ROUTE_DISCOVERY.md', 'verify-issue-1489'],
 'Makefile': ['scripts/qa/verify-issue-1489.sh'],
 'skills/AGENTS_ROUTE_DISCOVERY.md': [*(f'RD1489-{i}' for i in range(1,11)), 'issue-1489/README.md'],
 'docs/qa/issue-1489/README.md': [*(f'AC{i}' for i in range(1,8)), 'PASS WITH SKIPS', 'Executed failures remain FAIL', 'RESULTS.md'],
 'docs/route-solver.md': ['automatic-discovery-1489', '#1489', 'truncated'],
 'docs/frontend.md': ['#1489', 'AGENTS_ROUTE_DISCOVERY.md'],
 'docs/indexer-invariants.md': ['#1489', 'AGENTS_ROUTE_DISCOVERY.md'],
}
for name, terms in checks.items():
    body = Path(name).read_text()
    for term in terms:
        assert term in body, f'{name}: missing {term}'
print('Discovery, progress, invariants and checklist crosslinks PASS')
PY
}
start
for attempt in first retest; do
  run "$attempt live probe original cohort and privacy boundary" python3 scripts/qa/test-probe-route-discovery-1489.py
  run "$attempt fixed monitoring aggregate classification boundary" python3 scripts/qa/test-probe-route-monitoring-1489.py
  run "$attempt private preparation no-sign, custody and scope boundary" python3 scripts/qa/test-prepare-wallet-attribution-1483.py
  run "$attempt actual browser progress intent and useful output guards" bash scripts/with-node.sh -- node --test scripts/qa/test-browser-route-discovery-1489.cjs scripts/qa/test-browser-case-lifecycle.cjs
  selected "$attempt discovery resource/cohort/producer regressions" RUST rust
  selected "$attempt native deeper promotion and exact API intents" DB sockets
  selected "$attempt actual Swap/Trade progress, adoption and review" FRONTEND frontend
done
selected 'frontend types' TYPES bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/tsc -b --pretty false
selected 'Rust formatting' FORMAT cargo fmt --manifest-path indexer/Cargo.toml -- --check
run 'verifier syntax' bash -n scripts/qa/verify-issue-1489.sh
run 'discovery API, invariants and agent crosslinks' documentation
skip 'Deployed artifacts, sustained browser/API clocks and monitoring require separate actual receipts'
skip 'Owned funded native-three-hop, independent SQL and physical signing require separate actual receipts'
finish
