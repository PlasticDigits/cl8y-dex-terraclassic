#!/usr/bin/env bash
# Automated verification for GitLab #693 — /trade ticket Market default, flatten chrome, token wash.
#
# Proves (unit + docs; no chain required):
#   T1  Fresh ticket defaults Market (aria-selected, pay, Market CTA, no price input).
#   T2  Limit reachable; book Edit still selects Limit.
#   T3  Compact text tabs (not tab-glass pills / not full-width grid).
#   T4  Heading TokenLogo sibling; Select a pair has no logo.
#   T5  No leftover orange header wash.
#   T6  No Side / Top buy / Top sell TicketSection.
#   T7  One Market/Limit + Docs line; no inner h3 Market.
#   T8  Slippage chips only under Advanced.
#   T9  Quote extras / limit expiry+pre-submit under Advanced.
#   T10 Money semantics unchanged (bid=token1, GET route/solve, hybrid always-on).
#   T11 Docs/skills + make verify-issue-693.
#
# Refs: skills/AGENTS_FRONTEND_TRADE_TICKET_FLATTEN.md,
#       frontend-dapp/src/components/trade/TradeOrderTicket.tsx,
#       docs/frontend.md § Trade page — ticket flatten
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
echo "  GitLab #693 — Trade ticket Market default + flatten + wash"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: wash util + TradePage flatten + Market slippage + price chrome" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/tokenHeadingWash.test.ts \
    src/pages/TradePage.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.slippagePresets.test.tsx \
    src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx \
    src/components/trade/__tests__/LimitOrderPriceField.test.tsx \
    src/components/ui/__tests__/TokenLogo.test.tsx'

run_step "code: default Market; compact text tabs; no tab-glass pills (T1/T3)" \
  bash -c 'grep -q "const \[orderTab, setOrderTab\]" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    grep "const \[orderTab, setOrderTab\]" frontend-dapp/src/components/trade/TradeOrderTicket.tsx | grep -q "'\''market'\''" && \
    grep -qE "trade-order-text-tabs" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    grep -qE "trade-order-text-tab" frontend-dapp/src/index.css && \
    grep -qE "\\.trade-order-text-tab:focus-visible" frontend-dapp/src/index.css && \
    ! grep -qE "tab-glass" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    ! grep -qE "grid grid-cols-2 gap-1 rounded-2xl" frontend-dapp/src/components/trade/TradeOrderTicket.tsx'

run_step "code: heading logo + no orange wash (T4/T5/A3)" \
  bash -c 'grep -qE "trade-ticket-heading-logo" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    grep -qE "TokenLogo" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    grep -qE "useTokenHeadingWash" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    ! grep -qE "rgba\(251, 146, 60" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    ! grep -qE "innerHTML|dangerouslySetInnerHTML" frontend-dapp/src/components/trade/TradeOrderTicket.tsx frontend-dapp/src/utils/tokenHeadingWash.ts frontend-dapp/src/hooks/useTokenHeadingWash.ts && \
    grep -qE "resolveAllowedTokenLogoUri" frontend-dapp/src/hooks/useTokenHeadingWash.ts'

run_step "code: no Side chrome / TicketSection / Top buy (T6/T7)" \
  bash -c '! grep -qE "function TicketSection|function TicketStat" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    ! grep -qE "Top buy|Top sell" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    ! grep -qE "title=\"Side\"|title=\"Market\"|title=\"Limit\"" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    grep -qE "trade-order-mode-docs" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    ! grep -qE "<h3 className=\"text-xs font-semibold uppercase tracking-wide\">Market</h3>" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx && \
    ! grep -qE "Taker swap at" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx'

run_step "code: slippage + quote extras under Advanced (T8/T9)" \
  bash -c 'grep -qE "SlippageProtectionPresets" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx && \
    grep -qE "trade-market-slippage-presets" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx && \
    awk "/advancedOpen &&/,/!dockSubmit/" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx | grep -qE "SlippageProtectionPresets" && \
    grep -qE "trade-market-quote-extras" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx && \
    grep -qF "showDeviationChrome={false}" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    grep -qE "LimitOrderExpiryField" frontend-dapp/src/components/trade/TradeOrderTicket.tsx'

run_step "code: money semantics unchanged (T10)" \
  bash -c 'grep -qE "quoteCw20ViaRouteSolve" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx && \
    grep -qE "hybridParamsWithSubmitCap" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx && \
    ! grep -qE "trade-market-hybrid-toggle" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx && \
    grep -qE "TRADE_MONEY_CTA_CLASS" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    grep -qE "setOrderTab\('\''limit'\''\)" frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
    grep -qE "onSideChange\('\''bid'\''\)" frontend-dapp/src/components/trade/LimitOrderBidAskSideSelector.tsx && \
    grep -qE "idPrefix=\"limit-orders\"" frontend-dapp/src/pages/LimitOrdersPage.tsx'

run_step "docs: frontend.md T693 + amended T563-6" \
  bash -c 'grep -qE "trade-page-ticket-flatten" docs/frontend.md && \
    grep -qE "\\*\\*T693-1" docs/frontend.md && \
    grep -qE "\\*\\*T693-8" docs/frontend.md && \
    grep -qE "Compact text tabs" docs/frontend.md && \
    grep -qE "T563-6" docs/frontend.md && \
    grep -qE "trade-order-text-tab" docs/frontend.md'

run_step "docs: design-system + QA + skills + AGENTS.md" \
  bash -c 'grep -qE "#693" docs/design-system.md && \
    grep -qE "10.2.19" QA_TEMPLATE.md && \
    grep -qE "\\*\\*T693-1" skills/AGENTS_FRONTEND_TRADE_TICKET_FLATTEN.md && \
    grep -qE "make verify-issue-693" skills/AGENTS_FRONTEND_TRADE_TICKET_FLATTEN.md && \
    grep -qE "AGENTS_FRONTEND_TRADE_TICKET_FLATTEN|#693" skills/AGENTS_FRONTEND_TRADE_TICKET_HEADING.md && \
    grep -qE "AGENTS_FRONTEND_TRADE_TICKET_FLATTEN|#693" skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md && \
    grep -qE "AGENTS_FRONTEND_TRADE_TICKET_FLATTEN|#693" skills/AGENTS_FRONTEND_CHROME_NESTING.md && \
    grep -qE "AGENTS_FRONTEND_TRADE_TICKET_FLATTEN|#693" skills/AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md && \
    grep -qE "AGENTS_FRONTEND_TRADE_TICKET_FLATTEN" AGENTS.md && \
    grep -qE "make verify-issue-693" AGENTS.md'

run_step "design tokens + chrome nesting" \
  bash -c 'python3 scripts/check_design_tokens.py && python3 scripts/check_chrome_nesting.py'

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
echo "==> GitLab #693 verification passed"
