#!/usr/bin/env bash
# Forgejo #1256 — editable Swap receive on native and client routes.
# Skips are recorded separately: PASS with explicit skips never claims execution.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"
PASS=0
FAIL=0
SKIP=0
declare -a RESULTS=()

run_step() {
  local label="$1"
  shift
  printf '\n[%s]\n' "$label"
  if "$@"; then
    PASS=$((PASS + 1))
    RESULTS+=("PASS  $label")
  else
    FAIL=$((FAIL + 1))
    RESULTS+=("FAIL  $label")
  fi
}

skip_step() {
  SKIP=$((SKIP + 1))
  RESULTS+=("SKIP  $1: $2")
  printf '\n[SKIP] %s: %s\n' "$1" "$2"
}

printf '#1256 — editable receive / native reverse / offer-in submit\n'

if [[ -n "${VERIFY_ISSUE_1256_SKIP_FRONTEND_REASON:-}" ]]; then
  skip_step "frontend RTL + reverse services/math" "$VERIFY_ISSUE_1256_SKIP_FRONTEND_REASON"
elif [[ ! -x frontend-dapp/node_modules/.bin/vitest ]]; then
  skip_step "frontend RTL + reverse services/math" "frontend dependencies unavailable (install dependencies, then rerun)"
else
  run_step "frontend: receive/URL/guards RTL + canonical parser" \
    bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
      src/pages/SwapPage.queryParams.test.tsx \
      src/utils/__tests__/swapQueryParams.test.ts \
      src/utils/quoteDebounce.test.ts
  run_step "frontend: native/client reverse + wrap/unwrap/tax inverses + invoice compatibility" \
    bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
      src/services/terraclassic/router.reverse.test.ts \
      src/services/terraclassic/router.test.ts \
      src/services/terraclassic/__tests__/wrapMapper.test.ts \
      src/utils/__tests__/nativeTransferTax.test.ts \
      src/utils/communityTaxNetOut.test.ts \
      src/utils/payInvoice.test.ts
fi

run_step "docs: expanded Q711/Q713, reverse invariants, and agent/QA crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_SWAP_REVERSE_QUOTES.md
    test -f docs/qa/issue-1256/README.md
    for invariant in Q711-6 Q713-4 R1256-1 R1256-2 R1256-3 R1256-4 R1256-5 R1256-6; do
      grep -Fq "**${invariant}**" docs/frontend.md
    done
    for file in skills/AGENTS_FRONTEND_SWAP_URL_SYNC.md skills/AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md AGENTS.md docs/testing.md; do
      grep -Fq "AGENTS_FRONTEND_SWAP_REVERSE_QUOTES.md" "$file"
    done
    grep -Fq "amount_out" skills/AGENTS_FRONTEND_SWAP_URL_SYNC.md
    grep -Fq "W8" skills/AGENTS_FRONTEND_SWAP_REVERSE_QUOTES.md
    grep -Fq "W9" skills/AGENTS_FRONTEND_SWAP_REVERSE_QUOTES.md
    grep -Fq "routerMinReceiveBase" skills/AGENTS_FRONTEND_SWAP_REVERSE_QUOTES.md
    grep -Fq "PASS with explicit skips" docs/qa/issue-1256/README.md
    grep -Fq "verify-issue-1256" Makefile
  '
run_step "guard: chrome nesting" python3 scripts/check_chrome_nesting.py

if [[ -n "${VERIFY_ISSUE_1256_SKIP_BROWSER_REASON:-}" ]]; then
  skip_step "browser: mocked-LCD receive input/URL (2 workers)" "$VERIFY_ISSUE_1256_SKIP_BROWSER_REASON"
elif [[ ! -x frontend-dapp/node_modules/.bin/playwright ]]; then
  skip_step "browser: mocked-LCD receive input/URL (2 workers)" "Playwright dependencies unavailable (install dependencies/browser, then rerun)"
else
  run_step "browser: mocked-LCD receive input/URL (2 workers; no chain needed)" \
    env E2E_SWAP_RECEIVE_1256=1 PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-3026}" \
      bash scripts/with-node.sh --cwd frontend-dapp -- \
      ./node_modules/.bin/playwright test --project=e2e-smoke --workers=2 e2e/swap-receive-1256.spec.ts
fi

printf '\n'
printf '%s\n' "${RESULTS[@]}"
printf '\nPASS=%s FAIL=%s SKIP=%s\n' "$PASS" "$FAIL" "$SKIP"
if [[ "$FAIL" -ne 0 ]]; then
  printf 'Overall FAIL: executed checks failed.\n' >&2
  exit 1
fi
if [[ "$SKIP" -ne 0 ]]; then
  printf 'Overall PASS with explicit skips; skipped checks remain unverified.\n'
else
  printf 'Overall PASS: all automated #1256 checks executed successfully.\n'
fi
