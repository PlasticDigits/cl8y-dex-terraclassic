#!/usr/bin/env bash
# Automated verification for GitLab #663 — official CL8Y product links (footer).
#
# Proves (unit + docs + Playwright smoke; no chain required):
#   1. Allowlist helper accepts only the two pinned HTTPS origins.
#   2. Footer component: Homepage + Bridge href / target / rel / testids.
#   3. LegalFooterNotice suite still passes; navItems / CSP unchanged.
#   4. Docs/skills P663-1–P663-8 + footer-only rule.
#   5. Playwright e2e/footer-product-links-663.spec.ts (5 workers, skip-chain).
#
# Refs: skills/AGENTS_FRONTEND_PRODUCT_LINKS.md,
#       frontend-dapp/src/utils/cl8yProductLinks.ts,
#       docs/frontend.md § Official CL8Y product links
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
echo "  GitLab #663 — official CL8Y product links (footer)"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: allowlist + Cl8yProductLinks + LegalFooterNotice" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/cl8yProductLinks.test.ts \
    src/components/common/__tests__/Cl8yProductLinks.test.tsx \
    src/components/legal/__tests__/LegalFooterNotice.test.tsx

run_step "code: footer mounts product nav outside TermsGate" \
  bash -c 'grep -qE "Cl8yProductLinks" frontend-dapp/src/components/common/Layout.tsx && \
  awk "/<footer className=\"app-footer-shell\">/,/<\\/footer>/" frontend-dapp/src/components/common/Layout.tsx | grep -q "Cl8yProductLinks" && \
  ! awk "/<ConnectedTermsGate>/,/<\\/ConnectedTermsGate>/" frontend-dapp/src/components/common/Layout.tsx | grep -q "Cl8yProductLinks"'

run_step "code: no VITE product URL, iframe, or window.open" \
  bash -c '! grep -nE "import.meta.env.VITE_CL8Y|VITE_CL8Y_.*URL" \
    frontend-dapp/src/utils/cl8yProductLinks.ts \
    frontend-dapp/src/components/common/Cl8yProductLinks.tsx && \
  ! grep -nE "window\\.open|<iframe" frontend-dapp/src/components/common/Cl8yProductLinks.tsx && \
  ! grep -nE "card-glass|shell-panel" frontend-dapp/src/components/common/Cl8yProductLinks.tsx'

run_step "code: navItems stays in-app; CSP does not add apex/bridge" \
  bash -c '! grep -qiE "https://cl8y.com|https://bridge.cl8y.com|bridge.cl8y.com" frontend-dapp/src/components/common/navItems.ts && \
  ! grep -qE "https://cl8y.com|https://bridge.cl8y.com" frontend-dapp/viteCsp.ts && \
  grep -qE "form-action '\''self'\''" frontend-dapp/viteCsp.ts && \
  grep -qE "frame-ancestors '\''none'\''" frontend-dapp/viteCsp.ts && \
  grep -qE "CL8Y_PRODUCT_HOME_HREF = '\''https://cl8y.com/'\''" frontend-dapp/src/utils/cl8yProductLinks.ts && \
  grep -qE "CL8Y_PRODUCT_BRIDGE_HREF = '\''https://bridge.cl8y.com/'\''" frontend-dapp/src/utils/cl8yProductLinks.ts'

run_step "docs: frontend.md P663-1–P663-8" \
  bash -c 'grep -qE "official-cl8y-product-links" docs/frontend.md && \
  grep -qE "\*\*P663-1\*\*" docs/frontend.md && \
  grep -qE "\*\*P663-8\*\*" docs/frontend.md && \
  grep -qE "make verify-issue-663" docs/frontend.md && \
  grep -qE "#663" docs/frontend.md'

run_step "docs: testing.md + QA_TEMPLATE footer row" \
  bash -c 'grep -qE "verify-issue-663" docs/testing.md && \
  grep -qE "P663-1" docs/testing.md && \
  grep -qE "9.2.7" QA_TEMPLATE.md && \
  grep -qE "#663" QA_TEMPLATE.md'

run_step "skill: AGENTS_FRONTEND_PRODUCT_LINKS P663 + verify" \
  bash -c 'grep -qE "\*\*P663-1" skills/AGENTS_FRONTEND_PRODUCT_LINKS.md && \
  grep -qE "\*\*P663-8" skills/AGENTS_FRONTEND_PRODUCT_LINKS.md && \
  grep -qE "make verify-issue-663" skills/AGENTS_FRONTEND_PRODUCT_LINKS.md && \
  grep -qE "navItems.ts" skills/AGENTS_FRONTEND_PRODUCT_LINKS.md'

run_step "skill: risk / header / clickwrap / copy / nesting crosslinks" \
  bash -c 'grep -qE "AGENTS_FRONTEND_PRODUCT_LINKS|#663" skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md && \
  grep -qE "AGENTS_FRONTEND_PRODUCT_LINKS|#663" skills/AGENTS_FRONTEND_RESPONSIVE_HEADER.md && \
  grep -qE "AGENTS_FRONTEND_PRODUCT_LINKS|#663" skills/AGENTS_FRONTEND_CLICKWRAP.md && \
  grep -qE "AGENTS_FRONTEND_PRODUCT_LINKS|#663" skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md && \
  grep -qE "AGENTS_FRONTEND_PRODUCT_LINKS|#663" skills/AGENTS_FRONTEND_CHROME_NESTING.md'

run_step "AGENTS.md playbook + lint table #663" \
  bash -c 'grep -qE "AGENTS_FRONTEND_PRODUCT_LINKS" AGENTS.md && \
  grep -qE "verify-issue-663" AGENTS.md'

if [[ "${VERIFY_ISSUE_663_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright: footer product links (5 workers)] skipped (VERIFY_ISSUE_663_SKIP_E2E=1)"
  ok "playwright: skipped (VERIFY_ISSUE_663_SKIP_E2E=1)"
elif [ -x "$REPO_ROOT/frontend-dapp/node_modules/.bin/playwright" ]; then
  run_step "playwright: footer product links (5 workers)" \
    bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-30663}" \
      bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test \
        --project=e2e-smoke e2e/footer-product-links-663.spec.ts'
else
  echo ""
  echo "[playwright: footer product links (5 workers)] SKIP (no Playwright install)"
  ok "playwright: skipped (no Playwright install)"
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
echo "==> GitLab #663 verification passed"
