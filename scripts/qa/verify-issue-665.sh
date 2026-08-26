#!/usr/bin/env bash
# Automated verification for GitLab #665 — trader profile Share link.
#
# Proves (unit + docs; Playwright when LocalTerra is up):
#   1. Canonical URL helper rejects invalid ids and strips search/hash.
#   2. ShareLinkButton: share vs abort vs clipboard fallback + live region.
#   3. TraderPage shows Share on valid path (incl. 404/outage); hides on empty/invalid.
#   4. AddressRow still copies bech32; portfolio Share is /trader/{wallet}.
#   5. CopyButton regression still green.
#   6. Docs/skills TS-1–TS-13 crosslinked; no helmet / hard-coded prod origin.
#   7. Chrome nesting stays green.
#
# Refs: skills/AGENTS_FRONTEND_SHARE_LINK.md,
#       frontend-dapp/src/utils/sharePageLink.ts,
#       docs/frontend.md § Share link
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
echo "  GitLab #665 — trader profile Share link"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: share helper + ShareLinkButton + CopyButton" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/sharePageLink.test.ts \
    src/components/ui/__tests__/ShareLinkButton.test.tsx \
    src/components/ui/__tests__/CopyButton.test.tsx \
    src/utils/__tests__/copyToClipboard.test.ts'

run_step "frontend: TraderPage + PortfolioPage Share presence" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/TraderPage.test.tsx \
    src/pages/PortfolioPage.test.tsx'

run_step "code: Share on trader H1 row; no extra shell-panel wrap" \
  grep -qE 'data-testid="trader-share-link"' frontend-dapp/src/pages/TraderPage.tsx && \
  grep -qE 'ShareLinkButton' frontend-dapp/src/pages/TraderPage.tsx && \
  grep -qE 'buildCanonicalShareUrl' frontend-dapp/src/pages/TraderPage.tsx && \
  grep -qE 'window.location.origin' frontend-dapp/src/pages/TraderPage.tsx && \
  bash -c '! grep -nE "dex.cl8y.com" frontend-dapp/src/utils/sharePageLink.ts' && \
  bash -c '! grep -nE "VITE_PUBLIC_ORIGIN" frontend-dapp/src/utils/sharePageLink.ts frontend-dapp/src/components/ui/ShareLinkButton.tsx frontend-dapp/src/pages/TraderPage.tsx' && \
  bash -c '! grep -nE "react-helmet|Helmet" frontend-dapp/src/pages/TraderPage.tsx frontend-dapp/src/components/ui/ShareLinkButton.tsx' && \
  bash -c '! grep -nE "dangerouslySetInnerHTML" frontend-dapp/src/components/ui/ShareLinkButton.tsx frontend-dapp/src/utils/sharePageLink.ts'

run_step "code: clipboard fallback uses copyToClipboard only" \
  grep -qE 'copyToClipboard' frontend-dapp/src/utils/sharePageLink.ts && \
  bash -c '! grep -nE "clipboard.writeText" frontend-dapp/src/utils/sharePageLink.ts frontend-dapp/src/components/ui/ShareLinkButton.tsx frontend-dapp/src/pages/TraderPage.tsx'

run_step "code: portfolio Share is /trader not /portfolio" \
  grep -qE 'data-testid="portfolio-share-link"' frontend-dapp/src/pages/PortfolioPage.tsx && \
  grep -qE "kind: 'trader'" frontend-dapp/src/pages/PortfolioPage.tsx && \
  bash -c '! grep -nE "kind: .portfolio." frontend-dapp/src/pages/PortfolioPage.tsx'

run_step "guard: check_chrome_nesting.py" \
  python3 scripts/check_chrome_nesting.py

run_step "docs: frontend.md TS-1–TS-13 + Share vs address copy" \
  grep -qE 'share-link-button|#665' docs/frontend.md && \
  grep -qE '\*\*TS-1\*\*' docs/frontend.md && \
  grep -qE '\*\*TS-13\*\*' docs/frontend.md && \
  grep -qE 'Share vs address copy' docs/frontend.md && \
  grep -qE 'AGENTS_FRONTEND_SHARE_LINK' docs/frontend.md

run_step "skill: AGENTS_FRONTEND_SHARE_LINK" \
  grep -qE '\*\*TS-1' skills/AGENTS_FRONTEND_SHARE_LINK.md && \
  grep -qE 'make verify-issue-665' skills/AGENTS_FRONTEND_SHARE_LINK.md && \
  grep -qE 'copyToClipboard' skills/AGENTS_FRONTEND_SHARE_LINK.md

run_step "skill: copy / address / OG / portfolio / copy-cognitive crosslinks #665" \
  grep -qE 'AGENTS_FRONTEND_SHARE_LINK|#665' skills/AGENTS_FRONTEND_COPY_BUTTON.md && \
  grep -qE 'AGENTS_FRONTEND_SHARE_LINK|#665' skills/AGENTS_FRONTEND_ADDRESS_ROW.md && \
  grep -qE 'AGENTS_FRONTEND_SHARE_LINK|#665' skills/AGENTS_FRONTEND_OPENGRAPH.md && \
  grep -qE 'AGENTS_FRONTEND_SHARE_LINK|#665' skills/AGENTS_FRONTEND_PORTFOLIO.md && \
  grep -qE 'AGENTS_FRONTEND_SHARE_LINK|#665' skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md && \
  grep -qE 'AGENTS_FRONTEND_SHARE_LINK|#665' skills/AGENTS_FRONTEND_CHROME_NESTING.md

run_step "AGENTS.md playbook link #665" \
  grep -qE 'AGENTS_FRONTEND_SHARE_LINK|#665' AGENTS.md && \
  grep -qE 'verify-issue-665' AGENTS.md

run_step "docs: testing.md verify-issue-665 row" \
  grep -qE 'verify-issue-665' docs/testing.md && \
  grep -qE 'TS-1' docs/testing.md

if make -s has-localterra >/dev/null 2>&1; then
  run_step "playwright: trader-page share smoke (5 workers)" \
    bash -c 'PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-3015}" bash scripts/with-node.sh --cwd frontend-dapp -- \
      ./node_modules/.bin/playwright test --project=e2e-smoke e2e/trader-page.spec.ts -g "665"'
else
  echo ""
  echo "[playwright: trader-page share smoke (5 workers)]"
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
