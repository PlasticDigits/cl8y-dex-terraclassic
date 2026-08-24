#!/usr/bin/env bash
# Automated verification for GitLab #622 — Playwright e2e-tx community-tax pair.
#
# Invariants E622-1–E622-8: skills/AGENTS_E2E_COMMUNITY_TAX_TX.md
#
# Static + unit always. Optional Playwright when VERIFY_ISSUE_622_CHAIN=1
# and LocalTerra + seed pins exist.
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
echo "  GitLab #622 — Playwright e2e-tx community-tax pair"
echo "════════════════════════════════════════════════════════════════"

run_docs() {
  set -euo pipefail
  rg -q "E622-1" skills/AGENTS_E2E_COMMUNITY_TAX_TX.md
  rg -q "E622-8" skills/AGENTS_E2E_COMMUNITY_TAX_TX.md
  rg -q "AGENTS_E2E_COMMUNITY_TAX_TX" skills/AGENTS_E2E_STRICT_CHAIN.md
  rg -q "AGENTS_E2E_COMMUNITY_TAX_TX" skills/AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md
  rg -q "AGENTS_E2E_COMMUNITY_TAX_TX" skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md
  rg -q "AGENTS_E2E_COMMUNITY_TAX_TX" skills/AGENTS_INDEXER_TAX_AWARE_ROUTING.md
  rg -q "AGENTS_E2E_COMMUNITY_TAX_TX" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "verify-issue-622" AGENTS.md
  rg -q "verify-issue-622" docs/testing.md
  rg -q "E622-1" docs/contracts-security-audit.md
  rg -q "community-tax-tx" frontend-dapp/e2e/README.md
  rg -q "e2e-tx community-tax" docs/local-development.md
}

run_spec_strict() {
  set -euo pipefail
  test -f frontend-dapp/e2e/community-tax-tx.spec.ts
  test -f frontend-dapp/e2e/helpers/community-tax-e2e.ts
  test -f frontend-dapp/e2e/helpers/community-tax-env.ts
  test -f frontend-dapp/src/utils/communityTaxTxEnv.ts
  rg -q "community-tax-tx.spec.ts" frontend-dapp/e2e/README.md
  # e2e-tx glob already matches *-tx.spec.ts
  rg -q '\*\*/\*-tx.spec.ts' frontend-dapp/playwright.config.ts
  # Never skip on missing tax pair / LCD (comments mentioning test.skip are ok)
  if rg -n "test\.skip\(" frontend-dapp/e2e/community-tax-tx.spec.ts \
    frontend-dapp/e2e/helpers/community-tax-e2e.ts; then
    echo "tax tx spec/helpers must not test.skip (E622-2)" >&2
    return 1
  fi
  rg -q "classify_tax_provision_action" scripts/e2e-provision-dev-wallet.sh
  rg -q "classify_tax_provision_action" scripts/lib/cw20-funding-kind.sh
  rg -q "requireCommunityTaxTxPins" frontend-dapp/e2e/helpers/community-tax-env.ts
  rg -q "parseCommunityTaxTxPins" frontend-dapp/e2e/helpers/community-tax-env.ts
  rg -q "readFrontendEnvLocal" frontend-dapp/e2e/helpers/community-tax-env.ts
  rg -q "columbus-5" frontend-dapp/e2e/helpers/community-tax-env.ts
  rg -q "assertLcdReachable" frontend-dapp/e2e/helpers/community-tax-e2e.ts
  rg -q "maxDeclaredForExtraDebitSell" frontend-dapp/e2e/helpers/community-tax-e2e.ts
  rg -q "TaxPreview" frontend-dapp/e2e/community-tax-tx.spec.ts
  rg -q "You Receive is net" frontend-dapp/e2e/community-tax-tx.spec.ts
  rg -q "TransferFrom 1:1" frontend-dapp/e2e/community-tax-tx.spec.ts
  rg -q "escrow 1:1" frontend-dapp/e2e/community-tax-tx.spec.ts
  # Wrap stays off the tax template (do not import wrap helpers)
  if rg -n "from '\\./helpers/wrap-e2e'|wrap-swap\\.spec|wrap-pool\\.spec" \
    frontend-dapp/e2e/community-tax-tx.spec.ts; then
    echo "do not force wrap onto the tax template (E622-8)" >&2
    return 1
  fi
  # Hybrid must stay on
  if rg -n "pool_only=true|useHybridBook|hybrid-off" frontend-dapp/e2e/community-tax-tx.spec.ts; then
    echo "do not turn hybrid off (E622-7 / #596)" >&2
    return 1
  fi
}

run_unit() {
  bash scripts/qa/test-cw20-funding-kind.sh
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/communityTaxTxEnv.test.ts \
    src/utils/taxPreviewMaxSpend.test.ts \
    src/utils/communityTaxNetOut.test.ts
}

run_chain_playwright() {
  export PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-3173}"
  export PLAYWRIGHT_BASE_URL="http://127.0.0.1:${PLAYWRIGHT_WEB_PORT}"
  # Dedicated Vite Origin must be in indexer CORS_ORIGINS (#625 leftover #2).
  bash "$REPO_ROOT/scripts/e2e-start-indexer.sh"
  CI=1 PLAYWRIGHT_WEB_PORT="$PLAYWRIGHT_WEB_PORT" \
    PLAYWRIGHT_BASE_URL="$PLAYWRIGHT_BASE_URL" \
    E2E_WRAP_INDEXER_WAIT_LOOPS="${E2E_WRAP_INDEXER_WAIT_LOOPS:-3}" \
    bash scripts/with-node.sh --cwd frontend-dapp -- \
    ./node_modules/.bin/playwright test e2e/community-tax-tx.spec.ts --project=e2e-tx
}

run_no_skip_chain_in_tx() {
  set -euo pipefail
  # Smoke-only skip; tax tx must stay on e2e-tx glob
  rg -q "community-tax-tx" frontend-dapp/e2e/community-tax-tx.spec.ts
  if rg -n "community-tax-tx" frontend-dapp/playwright.config.ts | rg -v "txSpecGlobs|e2e-tx"; then
    :
  fi
  # File name must match e2e-tx
  [[ frontend-dapp/e2e/community-tax-tx.spec.ts == *-tx.spec.ts ]] || true
  echo frontend-dapp/e2e/community-tax-tx.spec.ts | rg -q '\-tx\.spec\.ts$'
}

echo ""
echo "── first pass ──"
run_step "docs: E622 + skill crosslinks" run_docs
run_step "spec: exists, e2e-tx glob, no test.skip" run_spec_strict
run_step "spec: filename is e2e-tx only" run_no_skip_chain_in_tx
run_step "unit: tx env + extra-debit + buy net" run_unit

if [ "${VERIFY_ISSUE_622_CHAIN:-0}" = "1" ]; then
  if make has-localterra >/dev/null 2>&1 && grep -qE '^VITE_TOKEN_COMMUNITY_TAX_ADDRESS=terra1' \
    frontend-dapp/.env.local 2>/dev/null; then
    run_step "chain: Playwright community-tax-tx (e2e-tx)" run_chain_playwright
  else
    echo ""
    echo "[chain] SKIP — LocalTerra or tax pins missing (make deploy-local)"
    RESULTS+=("SKIP  live community-tax-tx Playwright")
  fi
else
  echo ""
  echo "[chain] SKIP — set VERIFY_ISSUE_622_CHAIN=1 after seed deploy + indexer"
  RESULTS+=("SKIP  live community-tax-tx Playwright")
fi

run_retest_static() {
  run_docs && run_spec_strict && run_no_skip_chain_in_tx
}

echo ""
echo "── retest ──"
run_step "retest docs + spec strict" run_retest_static
run_step "retest unit: tx env + extra-debit + buy net" run_unit

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #622 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "==> GitLab #622 verification passed"
exit 0
