#!/usr/bin/env bash
# Automated verification for GitLab #578 — Open Graph / Twitter cards.
#
# Proves (unit + docs + greps; no LocalTerra / no wallet):
#   1. Production bake uses absolute https://dex.cl8y.com/og-image.png (F1, F3, A1–A7).
#   2. og-image.png is 1200×630 PNG < 5 MB; concept source is the square (F2, OG-3, OG-4).
#   3. nginx serves /og-image.png as a file; SPA routes fall back to index.html (F4, F5).
#   4. No request-host / query OG construction; no helmet; no SVG OG (A5–A7, OG-5, OG-6).
#   5. Docs/skills OG-1–OG-8 + #488 QA note no longer treat the typesetting card as live.
#
# Refs: skills/AGENTS_FRONTEND_OPENGRAPH.md,
#       docs/frontend.md § Open Graph / social cards,
#       frontend-dapp/viteOg.ts
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
echo "  GitLab #578 — Open Graph / Twitter cards"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: OG origin allowlist + production HTML bake" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/viteOg.test.ts \
    src/utils/__tests__/ogImage.test.ts'

run_step "code: production bake wired in vite.config (not request host)" \
  grep -qE 'og-absolute-meta' frontend-dapp/vite.config.ts && \
  grep -qE 'bakeProductionOgHtml' frontend-dapp/vite.config.ts && \
  grep -qE 'PUBLIC_ORIGIN_ALLOWLIST' frontend-dapp/viteOg.ts && \
  bash -c '! grep -nE "window\.location|X-Forwarded-Host|headers\.host" \
    frontend-dapp/viteOg.ts frontend-dapp/vite.config.ts frontend-dapp/index.html'

run_step "code: source index.html keeps relative image URLs + large card" \
  grep -qE 'property="og:image" content="/og-image.png"' frontend-dapp/index.html && \
  grep -qE 'name="twitter:image" content="/og-image.png"' frontend-dapp/index.html && \
  grep -qE 'name="twitter:card" content="summary_large_image"' frontend-dapp/index.html && \
  grep -qE 'og:image:width" content="1200"' frontend-dapp/index.html && \
  grep -qE 'og:image:height" content="630"' frontend-dapp/index.html && \
  bash -c '! grep -qE "twitter:site" frontend-dapp/index.html' && \
  bash -c '! grep -R -qE "react-helmet|HelmetProvider" frontend-dapp/index.html frontend-dapp/src frontend-dapp/package.json'

run_step "code: nginx PNG is a real file; SPA routes share index.html" \
  grep -qE 'location = /og-image.png' docker/frontend/nginx.conf && \
  grep -qE 'try_files \$uri =404' docker/frontend/nginx.conf && \
  grep -qE 'try_files \$uri \$uri/ /index.html' docker/frontend/nginx.conf && \
  grep -qE 'Cache-Control "no-cache"' docker/frontend/nginx.conf && \
  grep -qE 'X-Frame-Options DENY' docker/frontend/nginx.conf && \
  grep -qE 'X-Content-Type-Options nosniff' docker/frontend/nginx.conf

run_step "code: Coolify bakes allowlisted VITE_PUBLIC_ORIGIN" \
  grep -qE 'VITE_PUBLIC_ORIGIN=https://dex.cl8y.com' docker/frontend/Dockerfile && \
  grep -qE 'VITE_PUBLIC_ORIGIN=\$VITE_PUBLIC_ORIGIN' docker/frontend/Dockerfile

run_step "code: no SVG OG; concept is not the public crawler path" \
  test -f frontend-dapp/public/og-image.png && \
  test -f frontend-dapp/brand/community-opengraph-concept.png && \
  test ! -e frontend-dapp/public/community-opengraph-concept.png && \
  bash -c '! grep -qiE "og:image.*\.svg|twitter:image.*\.svg" frontend-dapp/index.html frontend-dapp/viteOg.ts'

run_step "docs: frontend.md Open Graph invariants OG-1–OG-8" \
  grep -qE 'open-graph-social-cards' docs/frontend.md && \
  grep -qE '\*\*OG-1\*\*' docs/frontend.md && \
  grep -qE '\*\*OG-8\*\*' docs/frontend.md && \
  grep -qE 'make verify-issue-578' docs/frontend.md

run_step "docs: design-system + #488 QA no longer claim typesetting OG" \
  grep -qE 'community medallion|#578' docs/design-system.md && \
  grep -qE 'https://dex.cl8y.com/og-image.png' docs/design-system.md && \
  bash -c '! grep -qE "typesetting" docs/design-system.md' && \
  grep -qE '#578' docs/qa/issue-488/README.md && \
  grep -qE 'community medallion' docs/qa/issue-488/README.md

run_step "docs: skill + AGENTS.md playbook #578" \
  grep -qE '\*\*OG-1' skills/AGENTS_FRONTEND_OPENGRAPH.md && \
  grep -qE '\*\*OG-8' skills/AGENTS_FRONTEND_OPENGRAPH.md && \
  grep -qE 'make verify-issue-578' skills/AGENTS_FRONTEND_OPENGRAPH.md && \
  grep -qE 'AGENTS_FRONTEND_OPENGRAPH' AGENTS.md && \
  grep -qE 'verify-issue-578' AGENTS.md && \
  grep -qE 'AGENTS_FRONTEND_OPENGRAPH' skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md && \
  grep -qE '#578' skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md

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
