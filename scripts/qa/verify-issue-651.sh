#!/usr/bin/env bash
# Automated verification for GitLab #651 — phone-width /tiers card + How it works.
#
# Proves (unit + docs; Playwright when LocalTerra is up):
#   1. TiersPage disconnected / register / governance / fee-label RTL.
#   2. Hold phrases stay formatTokenAmountAbbrev; I13 helpers unchanged.
#   3. No reserved empty w-28; register-tier-{id} testids; no *-neo.
#   4. Docs/skills T651-1–T651-8 + I15 crosslinked.
#   5. Optional: e2e/fee-tiers.spec.ts (5 workers, no e2e-tx).
#
# Refs: skills/AGENTS_FRONTEND_TIERS_PHONE.md,
#       frontend-dapp/src/pages/TiersPage.tsx,
#       docs/frontend.md § Tiers Page
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
echo "  GitLab #651 — /tiers phone-width layout"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: TiersPage disconnected / register / I4 / I13" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/TiersPage.test.tsx \
    src/utils/__tests__/formatAmount.test.ts \
    src/utils/__tests__/limitOrderFeeSummary.test.ts \
    src/utils/__tests__/feeDiscountUiCopy.test.ts'

run_step "code: no empty reserved Register slot; hold nowrap + register testids" \
  grep -qE 'data-testid=\{`tier-hold-\$\{tier_id\}`\}' frontend-dapp/src/pages/TiersPage.tsx && \
  grep -qE 'whitespace-nowrap' frontend-dapp/src/pages/TiersPage.tsx && \
  grep -qE 'data-testid=\{`register-tier-\$\{tier_id\}`\}' frontend-dapp/src/pages/TiersPage.tsx && \
  grep -qE 'tiers-how-it-works-mobile' frontend-dapp/src/pages/TiersPage.tsx && \
  grep -qE 'min-h-11' frontend-dapp/src/pages/TiersPage.tsx && \
  bash -c '! grep -nE "className=\"w-28\"" frontend-dapp/src/pages/TiersPage.tsx' && \
  bash -c '! grep -nE -- "-neo" frontend-dapp/src/pages/TiersPage.tsx' && \
  bash -c '! grep -qE "dangerouslySetInnerHTML" frontend-dapp/src/pages/TiersPage.tsx'

run_step "docs: frontend.md T651-1–T651-8 + QA 11.1.4" \
  grep -qE '\*\*T651-1\*\*' docs/frontend.md && \
  grep -qE '\*\*T651-8\*\*' docs/frontend.md && \
  grep -qE 'tiers-phone|#651' docs/frontend.md && \
  grep -qE 'Hold \{n\} CL8Y|#651' QA_TEMPLATE.md && \
  grep -qE '11.1.4' QA_TEMPLATE.md

run_step "docs: I15 + no duplicated numeric ladder in frontend.md" \
  grep -qE '\| I15 \|' docs/reference/fee-discount-tiers.md && \
  grep -qE 'AGENTS_FRONTEND_TIERS_PHONE' docs/reference/fee-discount-tiers.md && \
  bash -c '! grep -nE "7500000000000000000000" docs/frontend.md'

run_step "skill: AGENTS_FRONTEND_TIERS_PHONE" \
  grep -qE '\*\*T651-1' skills/AGENTS_FRONTEND_TIERS_PHONE.md && \
  grep -qE 'make verify-issue-651' skills/AGENTS_FRONTEND_TIERS_PHONE.md && \
  grep -qE 'whitespace-nowrap' skills/AGENTS_FRONTEND_TIERS_PHONE.md

run_step "skill: fee-discount + design + copy + #632 crosslinks" \
  grep -qE 'AGENTS_FRONTEND_TIERS_PHONE|#651' skills/AGENTS_FEE_DISCOUNT_TIERS.md && \
  grep -qE 'AGENTS_FRONTEND_TIERS_PHONE|#651' skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md && \
  grep -qE 'AGENTS_FRONTEND_TIERS_PHONE|#651' skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md && \
  grep -qE 'AGENTS_FRONTEND_TIERS_PHONE|#651' skills/AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md

run_step "AGENTS.md playbook link #651" \
  grep -qE 'AGENTS_FRONTEND_TIERS_PHONE|#651' AGENTS.md && \
  grep -qE 'verify-issue-651' AGENTS.md

run_step "docs: testing.md verify-issue-651 row" \
  grep -qE 'verify-issue-651' docs/testing.md && \
  grep -qE 'T651-1' docs/testing.md

if make -s has-localterra >/dev/null 2>&1; then
  run_step "playwright: fee-tiers desktop + phone (5 workers)" \
    bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- \
      ./node_modules/.bin/playwright test --project=e2e-smoke e2e/fee-tiers.spec.ts'
else
  echo ""
  echo "[playwright: fee-tiers desktop + phone (5 workers)]"
  echo "  SKIP (LocalTerra not up — unit + docs still required)"
  ok "playwright: skipped (no LocalTerra)"
fi

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  $PASS passed, $FAIL failed"
echo "────────────────────────────────────────────────────────────────"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
