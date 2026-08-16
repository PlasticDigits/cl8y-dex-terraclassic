#!/usr/bin/env bash
# Automated verification for GitLab #527 — /trade ticket money-CTA dock.
#
# Proves (unit + docs; Playwright when LocalTerra is up):
#   1. Footer is a flex sibling of the ticket scrollport (not position:sticky/fixed).
#   2. Limit + Market share trade-ticket-submit-footer; guards stay in flow.
#   3. RTL DOM order / no duplicate CTAs / Update price in footer.
#   4. Docs/skills T527-1–T527-10 crosslinked; AGENTS playbook present.
#   5. A9: footer chrome does not innerHTML indexer/wallet strings.
#
# Refs: skills/AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md,
#       frontend-dapp/src/components/trade/TradeTicketSubmitFooter.tsx,
#       docs/frontend.md § Trade page — ticket footer CTA
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
echo "  GitLab #527 — Trade ticket money-CTA dock"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: TradePage footer DOM + Market dock + invert" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/TradePage.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx'

run_step "code: footer sibling class, no sticky/fixed CTA" \
  grep -qE 'trade-ticket-submit-footer' frontend-dapp/src/components/trade/TradeTicketSubmitFooter.tsx && \
  grep -qE 'data-testid="trade-order-ticket-card"' frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
  grep -qE 'dockSubmit' frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx && \
  grep -qE '\.trade-ticket-submit-footer' frontend-dapp/src/index.css && \
  bash -c '! awk "/\\.trade-ticket-submit-footer/,/^  \\}/" frontend-dapp/src/index.css | grep -qE "position:\\s*(sticky|fixed)"' && \
  bash -c '! grep -qE "trade-limit-submit-sticky|trade-limit-sticky-clearance" frontend-dapp/src/index.css frontend-dapp/src/components/trade/TradeOrderTicket.tsx'

run_step "code: A5 no position:fixed / createPortal footer" \
  bash -c '! grep -qE "position:\s*fixed" frontend-dapp/src/components/trade/TradeTicketSubmitFooter.tsx frontend-dapp/src/components/trade/TradeOrderTicket.tsx' && \
  bash -c '! grep -qE "createPortal" frontend-dapp/src/components/trade/TradeTicketSubmitFooter.tsx frontend-dapp/src/components/trade/TradeOrderTicket.tsx'

run_step "code: A9 footer uses TxResultAlert text nodes, not innerHTML" \
  grep -qE 'TxResultAlert' frontend-dapp/src/components/trade/TradeTicketSubmitFooter.tsx && \
  bash -c '! grep -qE "innerHTML|dangerouslySetInnerHTML" frontend-dapp/src/components/trade/TradeTicketSubmitFooter.tsx frontend-dapp/src/components/trade/TradeOrderTicket.tsx'

run_step "code: Market dockSubmit publishes chrome; standalone still in-flow" \
  grep -qE 'onSubmitChromeChange' frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx && \
  grep -qE '!dockSubmit && <TradeMarketSubmitChrome' frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx

run_step "docs: frontend.md T527-1–T527-10" \
  grep -qE 'trade-page-ticket-footer-cta' docs/frontend.md && \
  grep -qE '\*\*T527-1\*\*' docs/frontend.md && \
  grep -qE '\*\*T527-10\*\*' docs/frontend.md

run_step "skill: AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK T527 + verify" \
  grep -qE '\*\*T527-1' skills/AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md && \
  grep -qE 'trade-ticket-submit-footer' skills/AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md && \
  grep -qE 'make verify-issue-527' skills/AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md

run_step "skill: #500 playbook no longer requires sticky-inside-scroll" \
  grep -qE 'trade-ticket-submit-footer' skills/AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md && \
  bash -c '! grep -qE "Keep the sticky CTA" skills/AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md'

run_step "skill: layout + header + AGENTS.md crosslinks #527" \
  grep -qE 'AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK|#527' skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md && \
  grep -qE 'AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK|#527' skills/AGENTS_FRONTEND_RESPONSIVE_HEADER.md && \
  grep -qE 'AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK|#527' AGENTS.md

run_step "e2e: trade-page-responsive.spec.ts asserts card-bottom alignment" \
  grep -qE 'trade-ticket-submit-footer' frontend-dapp/e2e/trade-page-responsive.spec.ts && \
  grep -qE 'ticket card bottom|ticket-card|trade-order-ticket-card' frontend-dapp/e2e/trade-page-responsive.spec.ts && \
  grep -qE 'GitLab #527' frontend-dapp/e2e/trade-page-responsive.spec.ts

if [ "${VERIFY_ISSUE_527_SKIP_E2E:-0}" = "1" ]; then
  echo ""
  echo "[e2e] SKIP — VERIFY_ISSUE_527_SKIP_E2E=1"
  RESULTS+=("SKIP  e2e Playwright (VERIFY_ISSUE_527_SKIP_E2E=1)")
elif make has-localterra >/dev/null 2>&1 && [ -f frontend-dapp/.env.local ]; then
  run_step "e2e: Playwright #527 geometry (Chromium)" \
    bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- \
      ./node_modules/.bin/playwright test e2e/trade-page-responsive.spec.ts \
      --project=e2e-smoke --retries=0 -g "GitLab #527|#500 / #527"'
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
echo "==> GitLab #527 verification passed"
