#!/usr/bin/env bash
# Automated verification for GitLab #531 — retail LUNC liquidity how-to.
#
# Proves (unit + docs; Playwright P1–P3 when the frontend can boot):
#   1. Static copy covers two-sided LP, wrap/auto-wrap, gas, withdraw, no incentive, limits ≠ LP.
#   2. Hint dismiss + <details> on /pool; no innerHTML; in-app links only (A2 / A7 / A10).
#   3. Footer + Portfolio discoverability; onboarding strip still Swap · Trade · Limits.
#   4. Skills/docs invariants H531-1–H531-10 crosslinked.
#
# Refs: skills/AGENTS_FRONTEND_POOL_LP_HOWTO.md,
#       frontend-dapp/src/utils/poolLpHowtoCopy.ts,
#       frontend-dapp/src/components/pool/PoolLpHowto.tsx
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
echo "  GitLab #531 — retail LUNC liquidity how-to"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: howto copy + storage + component + PoolPage + footer" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/poolLpHowtoCopy.test.ts \
    src/utils/__tests__/poolLpHowto.test.ts \
    src/components/pool/__tests__/PoolLpHowto.test.tsx \
    src/pages/PoolPage.test.tsx \
    src/components/legal/__tests__/LegalFooterNotice.test.tsx \
    src/components/common/__tests__/TradeOnboardingStrip.test.tsx'

run_step "code: PoolPage mounts PoolLpHowto; no innerHTML" \
  grep -qE 'PoolLpHowto' frontend-dapp/src/pages/PoolPage.tsx && \
  bash -c '! grep -qE "dangerouslySetInnerHTML|innerHTML" frontend-dapp/src/components/pool/PoolLpHowto.tsx frontend-dapp/src/utils/poolLpHowtoCopy.ts'

run_step "code: no APR/points/farm chrome in howto user strings" \
  bash -c '! grep -E "^export const POOL_LP_HOWTO_" frontend-dapp/src/utils/poolLpHowtoCopy.ts | grep -qiE "APR|APY|\\bpoints\\b|\\bfarm"'

run_step "code: onboarding strip still Swap · Trade · Limits only" \
  grep -qE 'Swap.*Trade.*Limits' frontend-dapp/src/components/common/TradeOnboardingStrip.tsx && \
  bash -c '! grep -qE "PoolLpHowto|/pool#lp-howto" frontend-dapp/src/components/common/TradeOnboardingStrip.tsx frontend-dapp/src/pages/WrapPage.tsx frontend-dapp/src/pages/SwapPage.tsx'

run_step "code: footer + portfolio same-origin howto links" \
  grep -qE 'pool-lp-howto-footer-link' frontend-dapp/src/components/legal/LegalFooterNotice.tsx && \
  grep -qE 'portfolio-lp-howto-link' frontend-dapp/src/components/portfolio/PortfolioLpOverviewSection.tsx && \
  grep -qE 'POOL_LP_HOWTO_HREF' frontend-dapp/src/components/legal/LegalFooterNotice.tsx

run_step "docs: frontend.md H531-1–H531-10 + retail doc" \
  grep -qE 'retail-lunc-liquidity-howto' docs/frontend.md && \
  grep -qE '\*\*H531-1\*\*' docs/frontend.md && \
  grep -qE '\*\*H531-10\*\*' docs/frontend.md && \
  grep -qE 'How to add LUNC liquidity' docs/user-lunc-liquidity.md && \
  grep -qE 'user-lunc-liquidity.md' docs/user-incident-faq.md

run_step "skill: AGENTS_FRONTEND_POOL_LP_HOWTO H531-1–H531-10" \
  grep -qE '\*\*H531-1' skills/AGENTS_FRONTEND_POOL_LP_HOWTO.md && \
  grep -qE '\*\*H531-10' skills/AGENTS_FRONTEND_POOL_LP_HOWTO.md && \
  grep -qE 'make verify-issue-531' skills/AGENTS_FRONTEND_POOL_LP_HOWTO.md

run_step "crosslinks: copy + onboarding + provide + wrap + AGENTS.md" \
  grep -qE 'AGENTS_FRONTEND_POOL_LP_HOWTO|#531' skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md && \
  grep -qE 'AGENTS_FRONTEND_POOL_LP_HOWTO|#531' skills/AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md && \
  grep -qE 'AGENTS_FRONTEND_POOL_LP_HOWTO|#531' skills/AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md && \
  grep -qE 'AGENTS_FRONTEND_POOL_LP_HOWTO|#531' skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md && \
  grep -qE 'AGENTS_FRONTEND_POOL_LP_HOWTO|#531' AGENTS.md

if [[ "${VERIFY_ISSUE_531_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright P1–P3] skipped (VERIFY_ISSUE_531_SKIP_E2E=1)"
else
  run_step "playwright: P1–P3 + dismiss + no wrap/limits lecture" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-30660}" PLAYWRIGHT_BASE_URL="http://127.0.0.1:${PLAYWRIGHT_WEB_PORT:-30660}" bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test e2e/pool-lp-howto-531.spec.ts --project=e2e-smoke --workers=5'
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
