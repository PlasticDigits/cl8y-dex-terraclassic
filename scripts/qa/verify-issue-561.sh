#!/usr/bin/env bash
# Automated verification for GitLab #561 — /trade flatten chrome, independent tape, no drag-resize.
#
# Proves (unit + docs; Playwright when LocalTerra is up):
#   1. Persistence helpers accept only '1'/'0'; corrupt values fall back.
#   2. Desktop workspace is CSS grid — no PanelResizeHandle / react-resizable-panels on TradePage.
#   3. PriceChart is not wrapped in card-glass; tape is a sibling of the chart column.
#   4. Docs/skills L561-1–L561-12 + AGENTS.md crosslinks.
#   5. P10 e2e asserts handles absent and hide-ticket expands chart.
#
# Refs: skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md,
#       frontend-dapp/src/components/trade/TradeDesktopWorkspace.tsx,
#       docs/frontend.md § Trade page — desktop workspace
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
echo "  GitLab #561 — Trade desktop layout (no drag-resize)"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: TradePage + workspace panel unit tests" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/TradePage.test.tsx \
    src/utils/__tests__/tradeWorkspacePanels.test.ts'

run_step "code: no react-resizable-panels / PanelResizeHandle on TradePage" \
  bash -c '! grep -qE "react-resizable-panels|PanelResizeHandle|PanelGroup" frontend-dapp/src/pages/TradePage.tsx frontend-dapp/src/components/trade/TradeDesktopWorkspace.tsx'

run_step "code: chart absorbs vacated side track (3.2fr)" \
  bash -c 'grep -qE "3.2fr" frontend-dapp/src/utils/tradeWorkspacePanels.ts && \
  grep -qE "3.2fr" frontend-dapp/src/utils/__tests__/tradeWorkspacePanels.test.ts'

run_step "code: visibility keys + inert ticket hide" \
  bash -c 'grep -qE "TRADE_BOOK_VISIBLE_KEY" frontend-dapp/src/utils/tradeWorkspacePanels.ts && \
  grep -qE "TRADE_TICKET_VISIBLE_KEY" frontend-dapp/src/utils/tradeWorkspacePanels.ts && \
  grep -qE "inert" frontend-dapp/src/components/trade/TradeDesktopWorkspace.tsx && \
  grep -qE "interactive" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
  grep -qE "trade-desktop-book-toggle" frontend-dapp/src/components/trade/TradeDesktopWorkspace.tsx && \
  grep -qE "trade-desktop-ticket-toggle" frontend-dapp/src/components/trade/TradeDesktopWorkspace.tsx && \
  grep -qE "trade-desktop-tape-panel" frontend-dapp/src/components/trade/TradeDesktopWorkspace.tsx'

run_step "code: TradeChartSlot does not wrap PriceChart in card-glass" \
  bash -c '! awk "/function TradeChartSlot/,/^export default function TradePage/" frontend-dapp/src/pages/TradePage.tsx | grep -q "card-glass"'

run_step "docs: frontend.md L561-1–L561-12" \
  bash -c 'grep -qE "trade-page-desktop-workspace" docs/frontend.md && \
  grep -qE "\*\*L561-1" docs/frontend.md && \
  grep -qE "\*\*L561-12" docs/frontend.md'

run_step "docs: design-system one chrome layer" \
  bash -c 'grep -qE "One chrome layer per region" docs/design-system.md && grep -qE "#561" docs/design-system.md'

run_step "skill: AGENTS_FRONTEND_TRADE_PAGE_LAYOUT L561 + verify" \
  bash -c 'grep -qE "\*\*L561-1" skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md && \
  grep -qE "make verify-issue-561" skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md && \
  ! grep -qE "Keep desktop.*react-resizable-panels" skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md'

run_step "skill: design-system + onboarding + AGENTS.md crosslinks #561" \
  bash -c 'grep -qE "#561" skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md && \
  grep -qE "#561" skills/AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md && \
  grep -qE "AGENTS_FRONTEND_TRADE_PAGE_LAYOUT|#561" AGENTS.md'

run_step "e2e spec: P10 asserts no resize handles" \
  bash -c 'grep -qE "GitLab #561" frontend-dapp/e2e/trade-page-responsive.spec.ts && \
  grep -qE "trade-ticket-resize-handle" frontend-dapp/e2e/trade-page-responsive.spec.ts && \
  ! grep -qE "mouse.down" frontend-dapp/e2e/trade-page-responsive.spec.ts'

run_step "design tokens" \
  python3 scripts/check_design_tokens.py

if [ "${VERIFY_ISSUE_561_SKIP_E2E:-0}" = "1" ]; then
  echo ""
  echo "[e2e] SKIP — VERIFY_ISSUE_561_SKIP_E2E=1"
  RESULTS+=("SKIP  e2e Playwright (VERIFY_ISSUE_561_SKIP_E2E=1)")
elif make has-localterra >/dev/null 2>&1 && [ -f frontend-dapp/.env.local ]; then
  run_step "e2e: Playwright #561 layout (Chromium)" \
    bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- \
      ./node_modules/.bin/playwright test e2e/trade-page-responsive.spec.ts \
      --project=e2e-smoke --retries=0 -g "GitLab #561"'
else
  echo ""
  echo "[e2e] SKIP — LocalTerra or frontend-dapp/.env.local not ready"
  echo "  Probe: make has-localterra"
  echo "  Provision: make setup-cloud-localterra"
  RESULTS+=("SKIP  e2e Playwright (no LocalTerra / .env.local)")
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
echo "==> GitLab #561 verification passed"
