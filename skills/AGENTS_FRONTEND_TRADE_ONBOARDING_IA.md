# Agent playbook: Trade onboarding IA, CTA hierarchy, progressive disclosure

Use when changing first-visit trade guidance, money-action button sizing on trade tickets, slippage preset touch targets, or default collapsed panels on `/trade` ([GitLab **#417**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/417)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade route — onboarding IA](../docs/frontend.md#trade-route-onboarding-ia) | Product invariants: onboarding strip, CTA class, slippage chips, collapsed tape/history |
| [`TradeOnboardingStrip.tsx`](../frontend-dapp/src/components/common/TradeOnboardingStrip.tsx) | Dismissible strip on `/`, `/trade`, `/limits` |
| [`tradeOnboarding.ts`](../frontend-dapp/src/utils/tradeOnboarding.ts) | `cl8y-dex-trade-onboarding-dismissed` |
| [`tradeWorkspacePanels.ts`](../frontend-dapp/src/utils/tradeWorkspacePanels.ts) | Tape + wallet-history expansion keys |
| [`tradeMoneyCta.ts`](../frontend-dapp/src/utils/tradeMoneyCta.ts) | `TRADE_MONEY_CTA_CLASS`, `TRADE_SLIPPAGE_PRESET_CLASS` |
| [`TradePage.tsx`](../frontend-dapp/src/pages/TradePage.tsx) | Desktop collapsible tape panel + disclosures |
| [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) | Swap CTA reference (`py-3.5 text-base`) |

## Rules of thumb

1. **Do not shrink primary money CTAs** below `TRADE_MONEY_CTA_CLASS` on limit place, market submit, or ladder place — secondary actions (View order, book row cancel) may stay compact.
2. **Onboarding dismiss must not block** wallet connect, swap submit, or pause/blacklist banners.
3. **Progressive disclosure is opt-in** — first visit collapses tape and wallet history only; chart, book, and order ticket stay visible.
4. **Persist panel prefs** in `localStorage` via `readTradePanelExpanded` / `writeTradePanelExpanded`; do not use session-only state.
5. **Mobile bottom nav** (`MOBILE_BOTTOM_NAV_ITEMS` in [`navItems.ts`](../frontend-dapp/src/components/common/navItems.ts)) must remain reachable — onboarding strip is in-page, not fixed over the tab bar.

## Regression tests

```bash
make test-frontend
# focused:
cd frontend-dapp && npm test -- --run \
  src/pages/TradePage.test.tsx \
  src/components/common/__tests__/TradeOnboardingStrip.test.tsx \
  src/utils/__tests__/tradeOnboarding.test.ts \
  src/utils/__tests__/tradeWorkspacePanels.test.ts
```

Indexer outage tests that assert tape copy must **expand** `trade-sub-lg-tape-disclosure` first when tape defaults collapsed.

## Related

- Trade workspace layout: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Swap route display / market quote row: [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md)
