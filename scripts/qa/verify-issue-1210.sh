#!/usr/bin/env bash
# Pinned invoice and creation sources share their ledger regression harness.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
pass=0 fail=0 skip=0
run() {
  local label="$1"; shift
  if "$@"; then echo "PASS: $label"; pass=$((pass+1));
  else echo "FAIL: $label"; fail=$((fail+1)); fi
}
run 'pinned paid-action parser boundaries' bash -c 'cd indexer && cargo test --locked --lib invoice_fees -- --test-threads=1'
if [[ -n "${VERIFY_ISSUE_1210_SKIP_DB_REASON:-}" ]]; then
  echo "SKIP: isolated ledger/API/catalog checks: $VERIFY_ISSUE_1210_SKIP_DB_REASON"; skip=$((skip+1))
else
  run 'ledger replay, sources, provenance, price/idle and daily semantics' bash -c \
    'cd indexer && cargo test --locked --test indexer_protocol_fees --test indexer_protocol_fees_series --test api_community_tokens -- --test-threads=1'
fi
if [[ -n "${VERIFY_ISSUE_1210_SKIP_UI_REASON:-}" ]]; then
  echo "SKIP: UI source-label checks: $VERIFY_ISSUE_1210_SKIP_UI_REASON"; skip=$((skip+1))
else
  run 'allowlisted labels and pin-removal retention' bash scripts/with-node.sh --cwd frontend-dapp -- \
    ./node_modules/.bin/vitest run src/components/protocol/ProtocolFeeStats.test.tsx
fi
run 'documented source and acceptance cross-links' python3 - <<'PY'
from pathlib import Path
for path in ('docs/qa/issue-1209/README.md', 'docs/qa/issue-1210/README.md',
             'docs/indexer-invariants.md', 'skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md'):
    assert '1354' in Path(path).read_text(), path
PY
run 'diff whitespace' git diff --check
echo "Results: $pass PASS, $fail FAIL, $skip SKIP; source/fixture scope only. Paid-chain replay is separately evidenced."
((fail == 0)) || exit 1
if ((skip > 0)); then echo 'PASS WITH SKIPS'; else echo 'PASS'; fi
