# Agent playbook: Limit order price field (trade + standalone page)

Use when changing **limit price** UX on `/trade` or `/limit-orders`: reference line from tape, % deviation, headline-scaled USD, submit validation, or the **Place limit** tooltip.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade page — limit order price field](../docs/frontend.md#trade-page-limit-order-price) | Invariants (reference, deviation colors, USD anchor, submit gate, tooltip) — [GitLab **#154**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/154) |
| [docs/limit-orders.md § dApp: retail form](../docs/limit-orders.md#dapp-retail-form-wires-invariants) | Cross-link to #154 bullet and pure helpers list |
| [`limitOrderPriceReference.ts`](../frontend-dapp/src/utils/limitOrderPriceReference.ts) | `tradeToToken1PerToken0Human`, deviation %, `anchorUsdForLimitPrice`, direction checks |
| [`limitOrderPricePlaceGate.ts`](../frontend-dapp/src/utils/limitOrderPricePlaceGate.ts) | `evaluateLimitOrderPricePlaceGate` — mirrors submit button + mutation throw |
| [`LimitOrderPriceField.tsx`](../frontend-dapp/src/components/trade/LimitOrderPriceField.tsx) | `LimitOrderPlaceLimitHeading`, `LimitOrderPriceInputWithContext` |
| [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) + [`TradePage.tsx`](../frontend-dapp/src/pages/TradePage.tsx) | Pass `indexerPair`, `latestTrade`, `tapeHeadlineUsd` |
| [`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx) | Same wiring with local `getPair` / `getTrades` queries |

## Rules of thumb

1. **Keep reference math in `limitOrderPriceReference.ts`** — UI components should not re-derive BigInt ratios inline.
2. **Tape headline string** passed to `anchorUsdForLimitPrice` must stay aligned with `PriceChart`’s `tapeLastPriceUsd` (usually `trades[0].price` from `getTrades`).
3. When changing submit rules, update **both** `evaluateLimitOrderPricePlaceGate` and the `placeMutation` throw path, plus Vitest under `utils/__tests__/limitOrderPrice*.test.ts`.
4. If copy or thresholds for “extreme deviation” change, update `docs/frontend.md` and this skill together.

## Related

- Trade workspace layout: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Parked / expired limits: [`AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](./AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md)
- Price chart / headline: [`AGENTS_FRONTEND_PRICE_CHART.md`](./AGENTS_FRONTEND_PRICE_CHART.md)
