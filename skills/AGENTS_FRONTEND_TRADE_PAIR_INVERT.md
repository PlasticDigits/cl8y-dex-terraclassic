# Agent playbook: Trade / Charts pair display invert (GitLab #524)

Audience: third-party agents touching `/trade` or `/charts` **Price (USD)**, pair pills, ticket **Buy {token}**, limit price fields, or market/limit submit on UST1 pairs.

**Issue:** [GitLab **#524**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524)  
**Invariants:** [`docs/frontend.md` § Trade pair display invert](../docs/frontend.md#trade-pair-display-invert) (**T524-1–T524-11**)  
**Related:** [#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522) factory USD of `asset_0`, [#466](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/466) quote-per-base, [#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508) UST1 secondary AMM (**U1**), [#151](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151) non-negative scale, [#226](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/226) finite candles.

## Problem class

After #522, UST1-as-`asset_0` pairs chart **USD of 1 UST1** (~$1). Traders need the **other token’s** USD (cUSTC / USTR) by default, a pair-orientation pill, and ticket **Buy {displayBase}** — without changing indexer math or on-chain `token1/token0`.

## Do / don’t

- **Do** invert in the frontend after `resolveTapeLastPriceUsd` / `indexerCandlesToChartPoints`.
- **Do** convert display prices with `displayPriceToFactoryToken1PerToken0` and sides with `factorySideFromDisplay` on every place / update-price / crossing gate / market quote+submit path.
- **Do** keep book **Edit** drafts in factory space; convert at the ticket edge.
- **Do** key invert by `pairAddr` (`sessionStorage` prefix `cl8y-dex-trade-pair-invert:`).
- **Don’t** change indexer `price` / `price_usd` / candles or aggregator `last_price`.
- **Don’t** reuse `LimitOrderSideFlipButton` as pair invert.
- **Don’t** invert on `PairSearchSelect` click (search must still open).
- **Don’t** silently invert `/limits` standalone.
- **Don’t** describe invert as mint/redeem (**U1**).
- **Don’t** pass `NaN` / `Infinity` / `≤ 0` reciprocals into lightweight-charts.
- **Don’t** `series.update()` historical bars when invert rewrites OHLC at the same times — that throws `Cannot update oldest data`. Use `setData` via `priceChartLightweightSeriesSync.ts`.

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/tradePairDisplayOrientation.ts` | `isUst1Leg`, `defaultDisplayInverted`, invert math, side/price convert, storage |
| `frontend-dapp/src/hooks/usePairDisplayOrientation.ts` | One state for `/trade` + `/charts` |
| `frontend-dapp/src/utils/pairPriceUsd.ts` | `resolveDisplayTapeLastPriceUsd` (factory USD unchanged) |
| `frontend-dapp/src/components/charts/priceChartCandles.ts` | `applyChartDisplayInvert` after finite mapping |
| `frontend-dapp/src/components/charts/priceChartLightweightSeriesSync.ts` | Invert historical rewrite → `setData` (not `update` from oldest) |
| `frontend-dapp/src/components/trade/PairDisplayInvertControls.tsx` | Pill + ticket icon |
| `frontend-dapp/src/components/trade/TradeOrderTicket.tsx` | No **Order ticket**; **Buy {displayBase}** + icon; factory convert on submit |
| `frontend-dapp/src/pages/TradePage.tsx` / `ChartsPage.tsx` | Shared hook + pill |

## Side map

| Display | Factory (UST1 = `asset_0`, inverted) |
|---------|--------------------------------------|
| Buy {other} | **ask** (pay UST1, receive other) |
| Sell {other} | **bid** (pay other, receive UST1) |
| Buy UST1 (not inverted) | **bid** |

## Regression

```bash
make verify-issue-524
```

Vitest: `tradePairDisplayOrientation.test.ts`, `pairPriceUsd.test.ts`, `priceChartCandles.test.ts`, `priceChartLightweightSeriesSync.test.ts` (invert → `setData`), `PriceChart.test.tsx`, `TradePage.test.tsx` (#524 describe), `TradeOrderTicket.invert.test.tsx`.

## Related

- [`AGENTS_FRONTEND_PRICE_CHART.md`](./AGENTS_FRONTEND_PRICE_CHART.md)
- [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — indexer math **unchanged**
- [`AGENTS_UST1_SECONDARY_AMM.md`](./AGENTS_UST1_SECONDARY_AMM.md) — **U1**
- [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md) — convert-on-submit
- [`AGENTS_LIMIT_PRICE_DECIMALS.md`](./AGENTS_LIMIT_PRICE_DECIMALS.md) — human↔raw decimal scale (#529) is **not** invert; apply raw→human **before** the #524 reciprocal
- [`AGENTS_FRONTEND_TOKEN_IDENTITY.md`](./AGENTS_FRONTEND_TOKEN_IDENTITY.md) — invert must not swap copy/explorer payloads (**T541-5**, [#541](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/541))
