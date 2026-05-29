# Agent playbook: accessibility CI (axe + critical routes)

Use when adding **retail-critical UI** on `/trade`, `/charts`, or the **header wallet** flows, or when changing **ARIA** on chart, order book, or order ticket widgets.

## Canonical references

| Doc / issue | Purpose |
|-------------|---------|
| [docs/frontend.md § Accessibility CI](../docs/frontend.md#accessibility-ci) | Invariants, local commands, exceptions ([GitLab **#214**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/214)) |
| [`frontend-dapp/e2e/a11y-critical-routes.spec.ts`](../frontend-dapp/e2e/a11y-critical-routes.spec.ts) | Playwright axe gate (`e2e-smoke` project) |
| [`frontend-dapp/e2e/helpers/a11y.ts`](../frontend-dapp/e2e/helpers/a11y.ts) | `assertNoCriticalA11yViolations` helper |
| [gaps/GAP_1780023683.md](../gaps/GAP_1780023683.md) | Gap inventory — Accessibility row |

## Invariants

1. **Canvas stays decorative** — `PriceChartLightweightCanvas` remains `aria-hidden`; announce interval + last price via `PriceChart` `role="region"` + `sr-only` `aria-live="polite"` summary ([§ Trade page — price chart](../docs/frontend.md#trade-page-price-chart-invariants)). **TradingView attribution** uses `layout.attributionLogo: false` plus the visible `price-chart-tradingview-attribution` link on `PriceChart` (in-chart `#tv-attr-logo` is focusable inside `aria-hidden`).
2. **Order book structure** — `OrderBookPanel` uses `<table>` / `<th scope="col">` per side; depth bars are `aria-hidden`; row actions keep `aria-label` with `order_id`.
3. **Order type tabs** — `TradeOrderTicket` limit/market: `tablist` + `tab` + `tabpanel` with `aria-controls` / `aria-labelledby`; pair-paused banner uses `role="alert"`.
4. **Wallet menu** — `role="menu"` wraps **menuitems only** (header `AddressRow` sits outside); focus moves to first `menuitem` on open and returns to trigger on Escape/close ([#214](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/214)).
5. **axe gate** — WCAG 2.1 A+AA tags; **zero critical/serious** on scanned routes. Canvas excluded via `[data-testid="price-chart-lightweight-canvas"] canvas` only.
6. **No silent rule disables** — document any `disableRules` in spec comments + `docs/frontend.md`.

## Related skills

- [`AGENTS_FRONTEND_A11Y_FORM_LABELS.md`](./AGENTS_FRONTEND_A11Y_FORM_LABELS.md) — form labels ([#143](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/143))
- [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md) — `:focus-visible` ([#144](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/144))
- [`AGENTS_FRONTEND_WALLET_CHIP.md`](./AGENTS_FRONTEND_WALLET_CHIP.md) — wallet chip + menu ([#186](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186), [#187](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/187))
- [`AGENTS_FRONTEND_PRICE_CHART.md`](./AGENTS_FRONTEND_PRICE_CHART.md) — lightweight-charts behavior

## Verify locally

```bash
cd frontend-dapp
npm run test:run -- src/components/charts/__tests__/PriceChart.test.tsx \
  src/components/trade/__tests__/OrderBookPanel.test.tsx \
  src/components/wallet/__tests__/WalletButton.test.tsx
PLAYWRIGHT_SKIP_CHAIN=1 npm run test:e2e:smoke -- e2e/a11y-critical-routes.spec.ts
```

Strict CI (LocalTerra + indexer) should also show the chart `region` before axe runs on `/trade` and `/charts`.
