#!/usr/bin/env bash
# Separate economic reporting floor and original-time reference diagnosis.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
ISSUE=1479
source scripts/qa/verify-followup-common.sh
rust() { cargo test --locked --manifest-path indexer/Cargo.toml --lib issue1479 -- --test-threads=1; }
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/utils/routeQuoteErrors.test.ts src/utils/__tests__/indexerErrors.test.ts \
    src/hooks/useQuoteErrorRecovery.test.tsx src/utils/localQuoteFreshness.test.tsx \
    src/utils/localRouteQuote.test.ts src/services/indexer/__tests__/client.test.ts
}
sockets() {
  [[ -n "${TEST_DATABASE_ADMIN_URL:-${TEST_DATABASE_URL:-}}" ]] || { echo 'Disposable QA capability unavailable; use explicit DB skip reason.' >&2; return 1; }
  local failed=0 suite
  for suite in small_market_reference_1479 quote_http_cache reference_cohort_1479 route_local_quotes_1452; do
    cargo test --locked --manifest-path indexer/Cargo.toml --test "$suite" -- --test-threads=1 || failed=1
  done
  return "$failed"
}
start
for attempt in first retest; do
  selected "$attempt original-time policy and readiness regressions" RUST rust
  selected "$attempt client reason recovery signing regressions" FRONTEND frontend
  selected "$attempt disposable socket zero-RPC and quote economics" DB sockets
done
selected 'frontend types' TYPES bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/tsc -b --pretty false
selected 'Rust formatting' FORMAT cargo fmt --manifest-path indexer/Cargo.toml -- --check
run 'verifier syntax' bash -n scripts/verify-issue-1479.sh scripts/qa/verify-followup-common.sh
skip 'Complete funded/native/attack matrix and live wallet/browser acceptance require separate actual receipts; policy/source tests do not satisfy complete deployed acceptance'
run 'invariant and checklist crosslinks' documentation
finish
