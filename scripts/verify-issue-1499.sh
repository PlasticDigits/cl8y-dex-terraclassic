#!/usr/bin/env bash
# Fresh display retention does not relax reviewed execution permission.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
ISSUE=1499
source scripts/qa/verify-followup-common.sh
rust() {
  cargo test --locked --manifest-path indexer/Cargo.toml --lib issue1499 -- --test-threads=1
}
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/vitest run \
    src/hooks/useQuotePricingDataPending.test.tsx src/hooks/useQuoteErrorRecovery.test.tsx \
    src/hooks/useSubmitAlignedSimQuote.test.ts src/utils/localQuoteFreshness.test.tsx \
    src/utils/__tests__/indexerErrors.test.ts src/pages/SwapPage.test.tsx \
    src/pages/SwapPage.queryParams.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx
}
start
for attempt in first retest; do
  selected "$attempt missing-versus-unsupported original observations" RUST rust
  selected "$attempt retention, intent races, expiry, negatives and recovery" FRONTEND frontend
done
selected 'frontend types' TYPES bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/tsc -b --pretty false
selected 'Rust formatting' FORMAT cargo fmt --manifest-path indexer/Cargo.toml -- --check
run 'verifier syntax' bash -n scripts/verify-issue-1499.sh
run 'lifecycle documentation' python3 -c "from pathlib import Path; assert 'AGENTS_QUOTE_LIFECYCLE.md' in Path('AGENTS.md').read_text(); assert 'issue-1499/README.md' in Path('skills/AGENTS_QUOTE_LIFECYCLE.md').read_text()"
skip 'Sustained attested live API/browser/funded scope requires separate real receipts'
finish
