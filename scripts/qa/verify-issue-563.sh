#!/usr/bin/env bash
# Automated verification for GitLab #563 — /trade ticket heading + Buy/Sell side colors.
#
# Proves (unit + docs; no chain required):
#   1. Full heading, no truncate, no compact wallet chip (disconnected + connected).
#   2. Footer Connect Wallet + header wallet stay; Place/Market stay btn-primary.
#   3. Buy/Sell use side-buy/side-sell; Limit/Market stay tab-glass; /limits compact matches.
#   4. Radiogroup keyboard + bid=Buy mapping (A1); heading is a text node (A2).
#   5. Docs/skills T563-1–T563-8 crosslinked; AGENTS playbook present.
#
# Refs: skills/AGENTS_FRONTEND_TRADE_TICKET_HEADING.md,
#       frontend-dapp/src/components/trade/LimitOrderBidAskSideSelector.tsx,
#       docs/frontend.md § Trade page — ticket heading
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
echo "  GitLab #563 — Trade ticket heading + Buy/Sell side colors"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: heading, chip, invert, side colors, limits" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/TradePage.test.tsx \
    src/components/trade/__tests__/LimitOrderBidAskSideSelector.test.tsx \
    src/components/trade/__tests__/TradeOrderTicket.invert.test.tsx \
    src/designTokens.test.ts && \
    bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/LimitOrdersPage.test.tsx -t "GitLab #563"'

run_step "code: heading wrap class, no truncate, header testid" \
  bash -c 'grep -qE "trade-ticket-heading" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    grep -qE "data-testid=\"trade-ticket-header\"" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    grep -qE "\\.trade-ticket-heading" frontend-dapp/src/index.css && \
    ! grep -n "trade-ticket-heading" frontend-dapp/src/components/trade/TradeOrderTicket.tsx | grep -q truncate && \
    ! awk "/\\.trade-ticket-heading/,/^  \\}/" frontend-dapp/src/index.css | grep -qE "ellipsis|truncate"'

run_step "code: compact wallet chip gone (T563-2 / A4)" \
  bash -c '! grep -qE "walletLabel" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    ! grep -qE "Connected wallet" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    ! grep -qE "address\\.slice\\(0, 8\\)" frontend-dapp/src/components/trade/TradeOrderTicket.tsx'

run_step "code: footer + header Connect Wallet remain (T563-3 / A3)" \
  bash -c 'grep -qE "TRADE_MONEY_CTA_CLASS" frontend-dapp/src/components/trade/TradeTicketSubmitFooter.tsx && \
    grep -qE "Connect Wallet" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    grep -qE "Connect Wallet" frontend-dapp/src/components/wallet/WalletButton.tsx && \
    grep -qE "btn-primary" frontend-dapp/src/utils/tradeMoneyCta.ts'

run_step "code: side-buy/side-sell + bid still onSideChange bid (A1 / A8)" \
  bash -c '  grep -qE "side-buy" frontend-dapp/src/components/trade/limitSideControlClass.ts && \
    grep -qE "side-sell" frontend-dapp/src/components/trade/limitSideControlClass.ts && \
    grep -qE "onSideChange\('\''bid'\''\)" frontend-dapp/src/components/trade/LimitOrderBidAskSideSelector.tsx && \
    grep -qE "onSideChange\('\''ask'\''\)" frontend-dapp/src/components/trade/LimitOrderBidAskSideSelector.tsx && \
    ! grep -qE "tab-glass-active" frontend-dapp/src/components/trade/LimitOrderBidAskSideSelector.tsx && \
    grep -qE "idPrefix=\"limit-orders\"" frontend-dapp/src/pages/LimitOrdersPage.tsx && \
    grep -qE "idPrefix=\"trade-ticket\"" frontend-dapp/src/components/trade/TradeOrderTicket.tsx'

run_step "code: Limit/Market tabs stay tab-glass; CTA stays btn-primary (T563-6)" \
  bash -c 'grep -qE "tab-glass-active" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    grep -qE "trade-order-tab-limit" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    grep -qE "TRADE_MONEY_CTA_CLASS" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    grep -qE "btn-primary btn-cta" frontend-dapp/src/utils/tradeMoneyCta.ts'

run_step "code: heading is a text node, not innerHTML (A2)" \
  bash -c '! grep -qE "innerHTML|dangerouslySetInnerHTML" frontend-dapp/src/components/trade/TradeOrderTicket.tsx frontend-dapp/src/components/trade/LimitOrderBidAskSideSelector.tsx'

run_step "code: side-control focus-visible; idle sell not alert-error (A5 / A6)" \
  bash -c 'grep -qE "\\.side-control:focus-visible" frontend-dapp/src/index.css && \
    ! awk "/\\.side-sell-idle/,/^  \\}/" frontend-dapp/src/index.css | grep -qE "alert-error" && \
    grep -qE -- "--side-buy-fg" frontend-dapp/src/theme-dark.css && \
    grep -qE -- "--side-buy-fg" frontend-dapp/src/theme-light.css && \
    grep -qE -- "--side-sell-fg" frontend-dapp/src/theme-dark.css && \
    grep -qE -- "--side-sell-fg" frontend-dapp/src/theme-light.css'

run_step "docs: frontend.md T563-1–T563-8" \
  bash -c 'grep -qE "trade-page-ticket-heading" docs/frontend.md && \
    grep -qE "\\*\\*T563-1" docs/frontend.md && \
    grep -qE "\\*\\*T563-8" docs/frontend.md && \
    grep -qE "side-buy-\\*" docs/frontend.md'

run_step "docs: design-system side-fill exception" \
  bash -c 'grep -qE "side-control" docs/design-system.md && \
    grep -qE "#563" docs/design-system.md && \
    grep -qE "Exception \\(#563\\)" docs/design-system.md'

run_step "skill: AGENTS_FRONTEND_TRADE_TICKET_HEADING T563 + verify" \
  bash -c 'grep -qE "\\*\\*T563-1" skills/AGENTS_FRONTEND_TRADE_TICKET_HEADING.md && \
    grep -qE "make verify-issue-563" skills/AGENTS_FRONTEND_TRADE_TICKET_HEADING.md && \
    grep -qE "side-buy" skills/AGENTS_FRONTEND_TRADE_TICKET_HEADING.md'

run_step "skill: side selector + design system + CTA dock + AGENTS.md crosslinks" \
  bash -c 'grep -qE "side-buy-\\*|side-control" skills/AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md && \
    grep -qE "AGENTS_FRONTEND_TRADE_TICKET_HEADING|#563" skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md && \
    grep -qE "AGENTS_FRONTEND_TRADE_TICKET_HEADING|#563" skills/AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md && \
    grep -qE "AGENTS_FRONTEND_TRADE_TICKET_HEADING|#563" skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md && \
    grep -qE "AGENTS_FRONTEND_TRADE_TICKET_HEADING|#563" skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md && \
    grep -qE "AGENTS_FRONTEND_TRADE_TICKET_HEADING" AGENTS.md && \
    grep -qE "make verify-issue-563" AGENTS.md'

run_step "design tokens script" \
  python3 scripts/check_design_tokens.py

if [[ "$FAIL" -gt 0 ]]; then
  echo ""
  echo "────────────────────────────────────────────────────────────────"
  echo "  $PASS passed, $FAIL failed"
  echo "────────────────────────────────────────────────────────────────"
  for r in "${RESULTS[@]}"; do
    echo "  $r"
  done
  exit 1
fi

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  $PASS passed, $FAIL failed"
echo "────────────────────────────────────────────────────────────────"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done
echo "==> GitLab #563 verification passed"
