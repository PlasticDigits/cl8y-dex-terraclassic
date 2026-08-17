#!/usr/bin/env bash
# Automated verification for GitLab #542 — Create Pair listed-CW20 picker + custom paste.
#
# Proves (unit + docs; Playwright P1–P5 when LocalTerra is up):
#   1. Catalog is bundled tokenlist CW20s + env overlays; natives/gems gated (T1–T6).
#   2. Create Pair page pick / paste / same-token / checksum / exclude / whitelist (T7–T17).
#   3. Swap TokenSearchSelect tests still pass (T18 / C8).
#   4. Docs/skills C542-1–C542-11 crosslinked; no factory/indexer API change.
#
# Refs: skills/AGENTS_FRONTEND_CREATE_PAIR_PICKER.md,
#       frontend-dapp/src/utils/createPairTokenCatalog.ts,
#       docs/frontend.md § Create pair listed CW20 picker
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

run_step() {
  local label="$1"
  shift
  echo ""
  echo "[$label]"
  if "$@"; then
    ok "$label"
  else
    bad "$label"
  fi
}

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #542 — Create Pair listed-CW20 picker"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: catalog + CreatePairPage + Swap TokenSearchSelect (T1–T18)" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/createPairTokenCatalog.test.ts \
    src/pages/CreatePairPage.test.tsx \
    src/components/trade/__tests__/TokenSearchSelect.test.tsx'

run_step "code: catalog is bundled tokenlist, not getAllTokens / fetch" \
  bash -c 'grep -qE "tokenlist/tokenlist.json" frontend-dapp/src/utils/createPairTokenCatalog.ts &&
    grep -qE "getCreatePairCw20Options" frontend-dapp/src/utils/createPairTokenCatalog.ts &&
    grep -qE "SOFT_LAUNCH_MINTABLE_TOKENS" frontend-dapp/src/utils/createPairTokenCatalog.ts &&
    ! grep -qE "getAllTokens" frontend-dapp/src/utils/createPairTokenCatalog.ts &&
    ! grep -qE "\\bfetch\\s*\\(" frontend-dapp/src/utils/createPairTokenCatalog.ts &&
    ! grep -qE "getAllTokens" frontend-dapp/src/pages/CreatePairPage.tsx &&
    ! grep -qE "getAllTokens" frontend-dapp/src/components/create/CreatePairTokenField.tsx'

run_step "code: Create Pair uses catalog + TokenSearchSelect + custom paste + same-address" \
  bash -c 'grep -qE "getCreatePairCw20Addresses" frontend-dapp/src/pages/CreatePairPage.tsx &&
    grep -qE "sameCreatePairAddress" frontend-dapp/src/pages/CreatePairPage.tsx &&
    grep -qE "useCodeIdCheck" frontend-dapp/src/pages/CreatePairPage.tsx &&
    grep -qE "UST1_CREATE_PAIR_SECONDARY_NOTICE" frontend-dapp/src/pages/CreatePairPage.tsx &&
    grep -qE "TokenSearchSelect" frontend-dapp/src/components/create/CreatePairTokenField.tsx &&
    grep -qE "listedCreatePairAddress" frontend-dapp/src/components/create/CreatePairTokenField.tsx &&
    grep -qE "Custom contract" frontend-dapp/src/components/create/CreatePairTokenField.tsx &&
    grep -qE "createPair\\(" frontend-dapp/src/pages/CreatePairPage.tsx'

run_step "code: no query prefill; factory createPair unchanged" \
  bash -c '! grep -qE "searchParams|useSearchParams|URLSearchParams" frontend-dapp/src/pages/CreatePairPage.tsx &&
    grep -qE "tokenAssetInfo\\(tokenA\\)" frontend-dapp/src/services/terraclassic/factory.ts &&
    grep -qE "create_pair" frontend-dapp/src/services/terraclassic/factory.ts'

run_step "docs: C542-1–C542-11 + address validation + tokenlist note" \
  bash -c 'grep -qE "create-pair-token-picker" docs/frontend.md &&
    grep -qE "\\*\\*C542-1\\*\\*" docs/frontend.md &&
    grep -qE "\\*\\*C542-11\\*\\*" docs/frontend.md &&
    grep -qE "C542-3" docs/frontend.md &&
    grep -qE "Create Pair picker reads this list" tokenlist/README.md'

run_step "skill: AGENTS_FRONTEND_CREATE_PAIR_PICKER + verify-issue-542" \
  bash -c 'grep -qE "\\*\\*C542-1" skills/AGENTS_FRONTEND_CREATE_PAIR_PICKER.md &&
    grep -qE "\\*\\*C542-11" skills/AGENTS_FRONTEND_CREATE_PAIR_PICKER.md &&
    grep -qE "make verify-issue-542" skills/AGENTS_FRONTEND_CREATE_PAIR_PICKER.md &&
    grep -qE "getAllTokens" skills/AGENTS_FRONTEND_CREATE_PAIR_PICKER.md'

run_step "crosslinks: token search + catalog rank + UST1 + copy + a11y + AGENTS.md" \
  bash -c 'grep -qE "AGENTS_FRONTEND_CREATE_PAIR_PICKER|#542" skills/AGENTS_FRONTEND_TOKEN_SEARCH.md &&
    grep -qE "AGENTS_FRONTEND_CREATE_PAIR_PICKER|#542" skills/AGENTS_FRONTEND_PAIR_CATALOG_RANK.md &&
    grep -qE "AGENTS_FRONTEND_CREATE_PAIR_PICKER|#542" skills/AGENTS_UST1_SECONDARY_AMM.md &&
    grep -qE "AGENTS_FRONTEND_CREATE_PAIR_PICKER|#542" skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md &&
    grep -qE "AGENTS_FRONTEND_CREATE_PAIR_PICKER|#542" skills/AGENTS_FRONTEND_A11Y_FORM_LABELS.md &&
    grep -qE "AGENTS_FRONTEND_CREATE_PAIR_PICKER|#542" AGENTS.md &&
    grep -qE "verify-issue-542" Makefile'

echo "── retest ──"
run_step "retest frontend unit T1–T18" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/createPairTokenCatalog.test.ts \
    src/pages/CreatePairPage.test.tsx \
    src/components/trade/__tests__/TokenSearchSelect.test.tsx'

if [[ "${VERIFY_ISSUE_542_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright P1–P5] skipped (VERIFY_ISSUE_542_SKIP_E2E=1)"
elif make has-localterra >/dev/null 2>&1; then
  if curl -sf --connect-timeout 2 --max-time 5 http://127.0.0.1:13170/cosmos/base/tendermint/v1beta1/node_info >/dev/null; then
    export E2E_LCD_PROXY_URL="${E2E_LCD_PROXY_URL:-http://127.0.0.1:13170}"
    export VITE_TERRA_LCD_URL="${E2E_LCD_PROXY_URL}"
    export E2E_LCD_URL="${E2E_LCD_PROXY_URL}"
  fi
  run_step "playwright: P1–P5 create-pair picker (e2e-smoke, 5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test \
      e2e/create-pair-picker-542.spec.ts --project=e2e-smoke'
else
  echo ""
  echo "[playwright P1–P5] make has-localterra failed (probe ran)."
  echo "  Provision: make setup-cloud-localterra"
  echo "  Then re-run: make verify-issue-542"
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  Results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for line in "${RESULTS[@]}"; do
  echo "  $line"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
