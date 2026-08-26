#!/usr/bin/env bash
# Automated verification for GitLab #659 — Swap direction seam plate.
#
# Proves (unit + docs; Playwright UI-only, no chain):
#   1. Opaque --swap-direction-surface in both themes.
#   2. Static occluder; no hover translate; :focus-visible ring.
#   3. Pay/Receive hairline kept; no extra card-glass / *-neo.
#   4. Docs/skills S659-1–S659-8 + QA 10.2.17.
#   5. Playwright computed-style (PLAYWRIGHT_SKIP_CHAIN=1, 5 workers).
#
# Refs: skills/AGENTS_FRONTEND_SWAP_DIRECTION_SEAM.md,
#       frontend-dapp/src/index.css,
#       docs/frontend.md § Swap direction seam
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
echo "  GitLab #659 — Swap direction seam plate"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: swapDirectionSeam token + CSS + markup" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/swapDirectionSeam.test.ts

run_step "design tokens (opaque plate in both themes)" \
  python3 scripts/check_design_tokens.py

run_step "chrome nesting allowlist unchanged" \
  python3 scripts/check_chrome_nesting.py

run_step "code: seam occluder + opaque plate + focus-visible; no hover translate" \
  bash -c 'grep -qF "swap-direction-seam" frontend-dapp/src/pages/SwapPage.tsx && \
  grep -qF "aria-label=\"Swap pay and receive tokens\"" frontend-dapp/src/pages/SwapPage.tsx && \
  grep -qF "pointer-events-none" frontend-dapp/src/pages/SwapPage.tsx && \
  grep -qF "pointer-events-auto" frontend-dapp/src/pages/SwapPage.tsx && \
  grep -qF "setFromToken(toToken)" frontend-dapp/src/pages/SwapPage.tsx && \
  ! grep -qF "hover:-translate-y" frontend-dapp/src/pages/SwapPage.tsx && \
  ! grep -qF "dangerouslySetInnerHTML" frontend-dapp/src/pages/SwapPage.tsx && \
  ! grep -qF -- "-neo" frontend-dapp/src/pages/SwapPage.tsx && \
  grep -qF "var(--swap-direction-surface)" frontend-dapp/src/index.css && \
  grep -qF ".swap-direction-seam::before" frontend-dapp/src/index.css && \
  grep -qF ".swap-direction-btn:focus-visible" frontend-dapp/src/index.css && \
  grep -qF -- "--swap-direction-surface: rgb(" frontend-dapp/src/theme-dark.css && \
  grep -qF -- "--swap-direction-surface: rgb(" frontend-dapp/src/theme-light.css && \
  grep -qF "border-bottom: 1px solid var(--chrome-border)" frontend-dapp/src/index.css'

run_step "code: Swap IO cards still card-glass (C653-5)" \
  bash -c 'grep -qE "card-glass swap-io-card-pay" frontend-dapp/src/pages/SwapPage.tsx && \
  grep -qE "card-glass swap-io-card-receive" frontend-dapp/src/pages/SwapPage.tsx'

run_step "docs: frontend.md S659-1–S659-8" \
  bash -c 'grep -qE "swap-direction-seam" docs/frontend.md && \
  grep -qE "\*\*S659-1\*\*" docs/frontend.md && \
  grep -qE "\*\*S659-8\*\*" docs/frontend.md && \
  grep -qE "#659" docs/frontend.md'

run_step "docs: design-system token + QA 10.2.17" \
  bash -c 'grep -qE "swap-direction-surface|#659" docs/design-system.md && \
  grep -qE "10.2.17" QA_TEMPLATE.md && \
  grep -qE "#659" QA_TEMPLATE.md'

run_step "skill: AGENTS_FRONTEND_SWAP_DIRECTION_SEAM" \
  bash -c 'grep -qE "\*\*S659-1" skills/AGENTS_FRONTEND_SWAP_DIRECTION_SEAM.md && \
  grep -qE "\*\*S659-8" skills/AGENTS_FRONTEND_SWAP_DIRECTION_SEAM.md && \
  grep -qE "make verify-issue-659" skills/AGENTS_FRONTEND_SWAP_DIRECTION_SEAM.md && \
  grep -qE "swap-direction-surface" skills/AGENTS_FRONTEND_SWAP_DIRECTION_SEAM.md'

run_step "skill: design + chrome + a11y + copy crosslinks" \
  bash -c 'grep -qE "AGENTS_FRONTEND_SWAP_DIRECTION_SEAM|#659" skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md && \
  grep -qE "AGENTS_FRONTEND_SWAP_DIRECTION_SEAM|#659" skills/AGENTS_FRONTEND_CHROME_NESTING.md && \
  grep -qE "swap-direction-btn|#659" skills/AGENTS_FRONTEND_A11Y_FOCUS.md && \
  grep -qE "AGENTS_FRONTEND_SWAP_DIRECTION_SEAM|#659" skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md'

run_step "AGENTS.md + testing.md playbook" \
  bash -c 'grep -qE "AGENTS_FRONTEND_SWAP_DIRECTION_SEAM|#659" AGENTS.md && \
  grep -qE "verify-issue-659" AGENTS.md && \
  grep -qE "verify-issue-659" docs/testing.md && \
  grep -qE "S659-1" docs/testing.md'

run_step "playwright: computed opaque plate (skip chain, 5 workers)" \
  bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-3174}" \
    bash scripts/with-node.sh --cwd frontend-dapp -- \
    ./node_modules/.bin/playwright test --project=e2e-smoke e2e/swap-direction-seam-659.spec.ts'

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
echo "==> GitLab #659 verification passed"
