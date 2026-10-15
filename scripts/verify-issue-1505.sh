#!/usr/bin/env bash
# Focused native recovery: no production mutation or request-triggered chain reads.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
ISSUE=1505
source scripts/qa/verify-followup-common.sh
rust() {
  cargo test --locked --manifest-path indexer/Cargo.toml --lib api::quote_trace -- --test-threads=1
  cargo test --locked --manifest-path indexer/Cargo.toml --lib api::local_wrapper_quote -- --test-threads=1
}
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/vitest run \
    src/utils/localQuoteFreshness.test.tsx src/hooks/useSubmitAlignedSimQuote.test.ts \
    src/hooks/useQuotePricingDataPending.test.tsx src/hooks/useQuoteErrorRecovery.test.tsx
}
qa() {
  node --test scripts/qa/test-live-input-clock-1499.cjs \
    scripts/qa/test-live-trading-lifecycle-1499.cjs scripts/qa/test-live-read-guard-1499.cjs \
    scripts/qa/test-live-quote-economics-1500.cjs
  python3 scripts/qa/test-native-recovery-1505.py
}
docs() {
  python3 - <<'PY'
from pathlib import Path
checks = {
 'AGENTS.md': ['AGENTS_NATIVE_QUOTE_RECOVERY.md', 'verify-issue-1505'],
 'skills/AGENTS_NATIVE_QUOTE_RECOVERY.md': [*(f'NR1505-{n}' for n in range(1,9)), 'issue-1505/README.md'],
 'docs/qa/issue-1505/README.md': [*(f'AC{n}' for n in range(1,7)), 'Executed failures', 'RESULTS.md'],
 'docs/indexer-invariants.md': ['#1505', 'AGENTS_NATIVE_QUOTE_RECOVERY.md'],
 'docs/route-solver.md': ['x-quote-request-id', 'NR1505'],
}
for file, terms in checks.items():
    body = Path(file).read_text()
    for term in terms: assert term in body, f'{file}: missing {term}'
print('Native recovery invariants and evidence crosslinks PASS')
PY
}
start
for attempt in first retest; do
  selected "$attempt original wrapper deadlines, partials, cancellation and correlation" RUST rust
  selected "$attempt synchronous original expiry and exact intent" FRONTEND frontend
  selected "$attempt browser clocks, native transport and probe evidence" QA qa
done
selected 'frontend types' TYPES bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/tsc -b --pretty false
selected 'Rust formatting' FORMAT cargo fmt --manifest-path indexer/Cargo.toml -- --check
run 'verifier syntax' bash -n scripts/verify-issue-1505.sh
run 'invariant/checklist links' docs
skip 'Focused exact-revision deployed socket/browser windows and monitoring require their separate live receipts'
finish
