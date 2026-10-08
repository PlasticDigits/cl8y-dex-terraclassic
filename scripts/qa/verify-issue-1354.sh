#!/usr/bin/env bash
# Source/copy checks and public reconciliation; an open accounting failure stays FAIL.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
pass=0 fail=0
run() {
  local label="$1"; shift
  if "$@"; then echo "PASS: $label"; pass=$((pass+1))
  else echo "FAIL: $label"; fail=$((fail+1)); fi
}
run 'invariant and QA cross-links' python3 - <<'PY'
from pathlib import Path
for file in ('AGENTS.md', 'docs/DEFILLAMA.md', 'docs/indexer-invariants.md',
             'docs/testing.md', 'skills/AGENTS_DEFILLAMA.md',
             'scripts/defillama/README.md'):
    assert '1354' in Path(file).read_text(), file
skill = Path('skills/AGENTS_DEFILLAMA.md').read_text()
for number in range(1, 11):
    assert f'DL1354-{number}' in skill
for file in ('docs/qa/issue-1354/README.md', 'docs/qa/issue-1354/verification.md',
             'scripts/qa/reconcile-defillama-fees-1354.sql'):
    assert Path(file).exists(), file
PY
run 'upstream copy integrity' python3 - <<'PY'
import hashlib, json
from pathlib import Path
receipt = json.loads(Path('docs/qa/issue-1354/upstream-volume.json').read_text())
for file, digest in receipt['copied_files_sha256'].items():
    assert hashlib.sha256(Path(file).read_bytes()).hexdigest() == digest, file
PY
run 'receipt null/rounding/unavailable boundaries' python3 scripts/qa/test-reconcile-defillama-1354.py
run 'volume recovery, scope, malformed data and unavailable HTTP' \
  node --test scripts/defillama/dexs/volume.test.js
run 'TVL, fees residual and stablecoin adapter regressions' \
  node --test scripts/defillama/tvl/tvlCore.test.js \
    scripts/defillama/dimensions/mapDaily.test.js scripts/defillama/stablecoins/ust1Core.test.js
run 'public completed-day drift (read-only; exit 1 findings, 2 unavailable)' \
  python3 scripts/qa/reconcile-defillama-1354.py \
    --output "${VERIFY_ISSUE_1354_OUTPUT:-/tmp/cl8y-defillama-1354.json}"
run 'diff whitespace' git diff --check
echo "Results: $pass PASS, $fail FAIL. Copy/regression PASS never establishes accounting acceptance."
((fail == 0))
