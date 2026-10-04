#!/usr/bin/env bash
# #1259: display-only Compact / Full preference. No chain or credentials required.
# Unavailable optional checks are recorded as SKIP; executed failures always fail.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
SKIP=0
RESULTS=()
run_step() {
  local label="$1"
  shift
  if "$@"; then
    PASS=$((PASS + 1))
    RESULTS+=("PASS: $label")
  else
    FAIL=$((FAIL + 1))
    RESULTS+=("FAIL: $label")
  fi
}
skip_step() {
  SKIP=$((SKIP + 1))
  RESULTS+=("SKIP: $1")
}

run_step "formatter / preference / consumers / Settings / signing and URL regressions (5 workers)" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run --maxWorkers=5 \
    src/utils/__tests__/formatAmount.test.ts \
    src/utils/numberDisplayPreference.test.tsx \
    src/utils/__tests__/numberDisplayFormatting.test.ts \
    src/utils/__tests__/formatProtocolStats.test.ts \
    src/utils/__tests__/tradeTapeDisplay.test.ts \
    src/utils/__tests__/chartsPairStats.test.ts \
    src/utils/__tests__/chartsOverviewStats.test.ts \
    src/utils/__tests__/pairPriceUsd.test.ts \
    src/utils/__tests__/swapQueryParams.test.ts \
    src/utils/__tests__/limitOrderExpectedReceive.test.ts \
    src/utils/__tests__/limitOrderNativeGasBalanceGate.test.ts \
    src/utils/__tests__/provideLiquidityNativeGasBalanceGate.test.ts \
    src/utils/__tests__/swapNativeGasBalanceGate.test.ts \
    src/pages/SwapPage.test.tsx \
    src/pages/TiersPage.test.tsx \
    src/pages/ProtocolPage.test.tsx \
    src/components/trade/__tests__/LimitOrderReceiveField.test.tsx \
    src/components/swap/__tests__/SwapPreSubmitSummary.test.tsx \
    src/components/trade/__tests__/LimitOrderPreSubmitSummary.test.tsx \
    src/components/pool/__tests__/PoolPreSubmitSummary.test.tsx

run_step "one chrome layer" python3 scripts/check_chrome_nesting.py

run_step "preference, chips, raw/price boundaries, and invariant crosslinks" python3 - <<'PY'
from pathlib import Path

def require(file, *terms):
    content = Path(file).read_text()
    for term in terms:
        assert term in content, f"{file}: missing {term!r}"
    return content

pref = require('frontend-dapp/src/utils/numberDisplayPreference.ts',
               'compact', 'full', 'cl8y-dex-number-display-mode',
               'readNumberDisplayMode', 'writeNumberDisplayMode', 'useNumberDisplayMode')
assert 'URLSearchParams' not in pref and 'dangerouslySetInnerHTML' not in pref
require('frontend-dapp/src/pages/SwapPage.tsx', 'swap-number-display-${mode}',
        'swap-number-display-preview', 'Number display')
require('frontend-dapp/src/utils/formatAmount.ts', 'formatTokenAmountGrouped',
        'formatPairPrice', 'PAIR_STATS_VOLUME_DISPLAY_CAP = 24')
# Display preferences cannot enter canonical URL or execute service modules.
for directory in ['frontend-dapp/src/services', 'frontend-dapp/src/types']:
    for file in Path(directory).rglob('*.ts'):
        assert 'numberDisplayPreference' not in file.read_text(), f'{file}: preference in raw path'
for file in ['frontend-dapp/src/utils/swapQueryParams.ts']:
    assert 'numberDisplayPreference' not in Path(file).read_text(), f'{file}: shareable preference'
require('docs/frontend.md', 'Q1259-1', 'Q1259-10', 'number-display-preference')
require('docs/testing.md', 'verify-issue-1259', 'PASS with SKIPs')
require('docs/README.md', 'AGENTS_FRONTEND_NUMBER_DISPLAY.md')
require('AGENTS.md', 'AGENTS_FRONTEND_NUMBER_DISPLAY.md', 'verify-issue-1259')
require('skills/AGENTS_FRONTEND_CHROME_NESTING.md', 'AGENTS_FRONTEND_NUMBER_DISPLAY.md')
require('skills/AGENTS_FRONTEND_NUMBER_DISPLAY.md', 'Q1259-1', 'PASS with SKIPs')
require('docs/qa/issue-1259/README.md', 'AC1', 'AC10', 'A1', 'A8', 'SKIP')
print('Preference / rendering boundaries and docs crosslinks present.')
PY

if [[ "${VERIFY_ISSUE_1259_BROWSER:-0}" == '1' ]]; then
  run_step "real Swap Settings browser smoke (mocked LCD reads; no chain)" \
    env PLAYWRIGHT_SKIP_CHAIN=1 E2E_SWAP_RECEIVE_1256=1 \
      PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-31259}" \
      bash scripts/with-node.sh --cwd frontend-dapp -- \
      ./node_modules/.bin/playwright test e2e/number-display-1259.spec.ts \
      --project=e2e-smoke --workers=5 --reporter=line
else
  skip_step "browser smoke not requested; run VERIFY_ISSUE_1259_BROWSER=1 make verify-issue-1259"
fi
skip_step "live multi-page / wallet / production visual checks: follow docs/qa/issue-1259/README.md when available"

printf '\n%s\n' "${RESULTS[@]}"
printf '\nResults: %s passed, %s failed, %s skipped\n' "$PASS" "$FAIL" "$SKIP"
if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
if [[ "$SKIP" -gt 0 ]]; then
  echo 'Overall PASS with SKIPs (unverified scope recorded above).'
else
  echo 'Overall PASS.'
fi
