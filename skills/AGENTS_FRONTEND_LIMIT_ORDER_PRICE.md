# Agent playbook: Limit order price field (trade + standalone page)

Use when changing **limit price** UX on `/trade` or `/limit-orders`: reference line from tape **or AMM pool**, % deviation, headline-scaled USD, submit validation, or the **Place limit** tooltip.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade page — limit order price field](../docs/frontend.md#trade-page-limit-order-price) | Invariants (reference, pool fallback, deviation, USD anchor, submit gate, tooltip) — [GitLab **#154**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/154), [**#166**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/166) |
| [docs/limit-orders.md § dApp: retail form](../docs/limit-orders.md#dapp-retail-form-wires-invariants) | Cross-link to #154 / #166 bullet and pure helpers list |
| [`limitOrderPriceReference.ts`](../frontend-dapp/src/utils/limitOrderPriceReference.ts) | `tradeToToken1PerToken0Human`, `resolveLimitOrderPriceRef`, `poolReservesToToken1PerToken0Human`, `pairDecimalsForLimitPriceRef`, deviation %, `anchorUsdForLimitPrice`, direction checks |
| [`useLimitOrderPriceRefBundle.ts`](../frontend-dapp/src/hooks/useLimitOrderPriceRefBundle.ts) | React Query: tape first, then LCD `getPool` when tape missing; exposes `refResolutionLoading` / `refResolutionError` for the place gate |
| [`limitOrderPricePlaceGate.ts`](../frontend-dapp/src/utils/limitOrderPricePlaceGate.ts) | `evaluateLimitOrderPricePlaceGate(side, price, ref, ctx?)` — mirrors submit button + mutation throw; **blocks** positive limits when ref unavailable (#166) |
| [`LimitOrderPriceField.tsx`](../frontend-dapp/src/components/trade/LimitOrderPriceField.tsx) | `LimitOrderPlaceLimitHeading`, `LimitOrderPriceInputWithContext` (receives resolved `refToken1PerToken0` + `refSource`) |
| [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) + [`TradePage.tsx`](../frontend-dapp/src/pages/TradePage.tsx) | Pass `indexerPair`, `latestTrade`, `tapeHeadlineUsd`; ticket runs `useLimitOrderPriceRefBundle` |
| [`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx) | Same hook + local `getPair` / `getTrades` queries |

## Rules of thumb

1. **Keep reference math in `limitOrderPriceReference.ts`** — UI components should not re-derive BigInt ratios inline.
2. **Tape headline string** passed to `anchorUsdForLimitPrice` must stay aligned with `PriceChart`’s `tapeLastPriceUsd` (usually `trades[0].price` from `getTrades`). Pool-only refs may leave headline USD as **—** until tape returns.
3. When changing submit rules, update **both** `evaluateLimitOrderPricePlaceGate` and the `placeMutation` throw path, plus Vitest under `utils/__tests__/limitOrderPrice*.test.ts`.
4. If copy or thresholds for “extreme deviation” change, update `docs/frontend.md` and this skill together.
5. **#166 invariant:** never allow a **positive** typed limit to submit without a resolved reference (tape or pool), unless product explicitly changes that contract.

## Related

- Limit **Bid / Ask** side control (button radiogroup): [`AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md`](./AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md) ([GitLab **#153**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/153))
- Trade workspace layout: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Parked / expired limits: [`AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](./AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md)
- Price chart / headline: [`AGENTS_FRONTEND_PRICE_CHART.md`](./AGENTS_FRONTEND_PRICE_CHART.md)
