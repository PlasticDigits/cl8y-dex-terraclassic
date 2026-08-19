# Agent playbook: Charts pair 24h Stats + TWAP human scale (GitLab #564)

Audience: third-party agents touching `/charts` pair **24h Stats**, **TWAP Oracle**, `/trade` or `/charts` candle **volume histogram**, or indexer `GET /api/v1/pairs/{addr}/stats`.

**Issue:** [GitLab **#564**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/564)  
**Invariants:** [`docs/frontend.md` § Charts pair 24h Stats](../docs/frontend.md#charts-pair-24h-stats) (**S564-1–S564-11**)  
**Oracle:** [`docs/twap-oracle.md`](../docs/twap-oracle.md) (arithmetic cumulative Decimal, not geometric ticks)

## Problem class

Pair 24h **Vol (UST1)** / **Vol (USTR)** and **TWAP 5m/1h/24h** passed raw indexer / CosmWasm Decimal integers to `formatNum`. UST1(6)/USTR(18) printed **385.8M**, **36T**, **111T** for ordinary hundreds / tens-of-thousands / ~111 quote-per-base. Catalog vol badges (#534) and overview USD (#548) were already human; pair Stats and TWAP were not.

## Do / don’t

- **Do** scale `volume_base` / `volume_quote` with **that pair row’s** `asset_0.decimals` / `asset_1.decimals` via [`formatPairStatsVolume`](../frontend-dapp/src/utils/formatAmount.ts).
- **Do** show **Vol (USD)** from indexer `volume_usd` via [`formatIndexedVolumeUsd`](../frontend-dapp/src/utils/chartsOverviewStats.ts). Unpriced + trades > 0 → `—`. Idle 0 trades → `$0`.
- **Do** humanize TWAP with `raw × 10^(d0 − d1)` ([`rawLimitPriceToHuman`](../frontend-dapp/src/utils/limitOrderPriceScale.ts)) then [`formatPairPrice`](../frontend-dapp/src/utils/formatAmount.ts). Labels stay **TWAP 5m/1h/24h**.
- **Do** format High/Low/Open/Close (USD) with `formatPairPrice` (or `$` + that), never compact `T`.
- **Do** scale candle histogram quote (else base) volume by the same pair-leg decimals. Invert still does **not** flip volume (**C543-8**).
- **Don’t** change indexer JSON units. `volume_base` / `volume_quote` stay raw. Do not add a human volume field.
- **Don’t** `formatNum` raw 6-dec or 18-dec volume or raw TWAP Decimal.
- **Don’t** treat TWAP as USD or `1/x` of `price_usd`. Do not apply #524 invert to 24h Stats USD OHLC / TWAP.
- **Don’t** invent pair USD from token volume × last price. Only `stats.volume_usd`.
- **Don’t** default missing decimals with `getDecimals` (6). Out of `0…18` → `—`.
- **Don’t** change on-chain oracle accumulation or CG/CMC raw volumes. Tape amounts are [#557](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557).

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/formatAmount.ts` | `formatPairStatsVolume`, `isPairLegDecimals` |
| `frontend-dapp/src/utils/chartsPairStats.ts` | `formatTwapHumanPrice`, `formatPairStatsUsdOhlc` |
| `frontend-dapp/src/utils/limitOrderPriceScale.ts` | Human scale `10^(d0−d1)` |
| `frontend-dapp/src/services/terraclassic/oracle.ts` | `computeTwapPriceDecimalString` (raw Decimal) |
| `frontend-dapp/src/pages/ChartsPage.tsx` | 24h Stats + TWAP StatBoxes |
| `frontend-dapp/src/components/charts/priceChartCandles.ts` | Histogram human scale |
| `frontend-dapp/src/components/charts/PriceChart.tsx` | Passes pair-leg decimals into the histogram |
| `indexer/src/api/pairs.rs` | `PairStatsResponse` (keep raw; `volume_usd` already present) |

## Regression

```bash
make verify-issue-564
```

Vitest: `formatAmount.test.ts` (`formatPairStatsVolume`), `chartsPairStats.test.ts`, `oracle.test.ts`, `ChartsPage.test.tsx` (#564 block), `priceChartCandles.test.ts` histogram scale.

## Related

- [`AGENTS_FRONTEND_CHARTS_OVERVIEW.md`](./AGENTS_FRONTEND_CHARTS_OVERVIEW.md) — overview 24h Volume (USD) only (#548)
- [`AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](./AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) — catalog / `/pool` vol badges ≠ Charts 24h Stats (#534)
- [`AGENTS_FRONTEND_USD_CANDLE_INVERT.md`](./AGENTS_FRONTEND_USD_CANDLE_INVERT.md) — volume is not inverted as price (#543)
- [`AGENTS_LIMIT_PRICE_DECIMALS.md`](./AGENTS_LIMIT_PRICE_DECIMALS.md) — same `10^(d0−d1)` scale as TWAP / limit price (#529)
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — factory `*_usd` OHLC (#522)
- [`AGENTS_FRONTEND_PRICE_CHART.md`](./AGENTS_FRONTEND_PRICE_CHART.md) — histogram wiring
