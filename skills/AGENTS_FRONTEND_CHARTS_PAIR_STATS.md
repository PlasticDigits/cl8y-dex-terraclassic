# Agent playbook: Charts pair 24h Stats — USD layout + human token/TWAP (GitLab #565 / #564)

Audience: third-party agents touching `/charts` **pair 24h Stats**, **TWAP Oracle**, `/trade` or `/charts` candle **volume histogram**, or indexer `GET /api/v1/pairs/{addr}/stats`.

**Issues:** [GitLab **#565**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/565) (Vol USD primary layout) · [GitLab **#564**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/564) (TWAP / histogram / USD OHLC human scale)  
**Invariants:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (rows **Charts pair 24h volume #565** and **Charts pair 24h Stats #564**), [`docs/frontend.md`](../docs/frontend.md) § [Charts pair 24h stats](../docs/frontend.md#charts-pair-stats)  
**Oracle:** [`docs/twap-oracle.md`](../docs/twap-oracle.md) (arithmetic cumulative Decimal, not geometric ticks)

## Problem class

Same `formatNum(raw)` bug as tape Amount ([#557](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557)), picker volume ([#534](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534)), and overview USD ([#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548)). Pair-level **Vol (token)** was left on production after #540 / #544 **AC4**: UST1/cUSTC showed **847.0M / 157.5B** for ~847 UST1 / ~157K cUSTC. UST1/USTR 18-dec quote and TWAP printed **`T`**.

Indexer already returns human `volume_usd` and raw `volume_base` / `volume_quote`. Do **not** humanize those JSON fields server-side.

## UI layout (#565)

[`ChartsPage.tsx`](../frontend-dapp/src/pages/ChartsPage.tsx) **24h Stats** strip (`charts-pair-24h-stats`):

| Row | Boxes |
|-----|-------|
| 1 (`grid-cols-2 md:grid-cols-4`) | **Vol (USD)** first, Trades, Price Change, High/Low/Open/Close (USD) |
| 2 (`grid-cols-2 mt-3`) | Vol (base symbol), Vol (quote symbol) |

Render only when `stats && activePair && activePair.pair_address === activePairAddr`.

**TWAP Oracle** section (below stats): `formatTwapHumanPrice` + `charts-twap-5m` / `1h` / `24h`.

**PriceChart:** pass `volumeBaseDecimals` / `volumeQuoteDecimals` for candle histogram ([#564 **S564-8**]).

## Invariants (P565-1–P565-7)

| ID | Rule |
|----|------|
| **P565-1** | Pair 24h stats **primary** volume is **Vol (USD)** = [`formatIndexedVolumeUsd`](../frontend-dapp/src/utils/chartsOverviewStats.ts)`(stats.volume_usd, stats.trade_count)`. `$` + compact. `charts-pair-volume-usd`. Tooltip: `24h volume in USD`. |
| **P565-2** | Never `formatNum(stats.volume_base)` or `formatNum(stats.volume_quote)`. |
| **P565-3** | Secondary **Vol ({symbol})** uses [`formatChartsPairTokenVolume`](../frontend-dapp/src/utils/chartsPairStats.ts) with **that pair’s** `asset_0` / `asset_1` decimals. `charts-pair-volume-base` / `charts-pair-volume-quote`. |
| **P565-4** | Unpriced / invalid `volume_usd` with `trade_count > 0` → `—`. Idle (`trade_count === 0` and USD `0`) → `$0`. |
| **P565-5** | [#524](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524) invert does **not** change USD or swap leg decimals on stats. |
| **P565-6** | Integrator JSON keeps raw `volume_base` / `volume_quote`. `volume_usd` stays human USD. |
| **P565-7** | Token vols render only for the selected pair that fetched stats. Hostile strings are text-only. |

## Invariants (S564-1–S564-11)

| ID | Rule |
|----|------|
| **S564-1** | Token vols scale raw sums with **that pair row's** `asset_*.decimals` via `formatChartsPairTokenVolume`. |
| **S564-2** | 18-dec quote volume is tens of thousands (`K` OK). Compact `T` only if **human** ≥ 1e12. |
| **S564-3** | Equal-decimal pairs (6/6) are not extra-scaled by 1e6 or 1e12. |
| **S564-4** | **Vol (USD)** is indexer `volume_usd` via `formatIndexedVolumeUsd`. Do not invent USD in the client. |
| **S564-5** | TWAP is human factory token1-per-token0: `raw × 10^(d0 − d1)` then `formatPairPrice`. Not USD. |
| **S564-6** | Same-decimal TWAP is identity in magnitude. |
| **S564-7** | High/Low/Open/Close (USD) use factory `*_usd` + [`formatPairStatsUsdOhlc`](../frontend-dapp/src/utils/chartsPairStats.ts). `charts-pair-*-usd`. |
| **S564-8** | Candle histogram scales quote volume by quote decimals (else base). Invert does not flip volume (**C543-8**). |
| **S564-9** | Indexer JSON units unchanged. No human-volume field. |
| **S564-10** | Missing / out-of-range decimals (`0…18`) or junk → `—`. |
| **S564-11** | Display only — not settlement. Tape amounts stay [#557](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557). |

## Do / don’t

- **Do** reuse `formatIndexedVolumeUsd` for USD. **Do** use `formatChartsPairTokenVolume` for token vols (not `formatPairStatsVolume` in Charts UI).
- **Do** humanize TWAP with `raw × 10^(d0 − d1)` ([`rawLimitPriceToHuman`](../frontend-dapp/src/utils/limitOrderPriceScale.ts)) then [`formatPairPrice`](../frontend-dapp/src/utils/formatAmount.ts).
- **Do** keep secondary token boxes on row 2 so USD is the default read.
- **Don’t** `formatNum` raw volume or TWAP. **Don’t** treat TWAP as USD. **Don’t** apply #524 invert to stats USD OHLC / TWAP.
- **Don’t** implement tape Amount (#557), overview strip (#548), or picker badges (#544) here.
- **Don’t** convert DEX volume with vFDUSD (**X4**).

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/chartsPairStats.ts` | `formatChartsPairTokenVolume`, `formatTwapHumanPrice`, `formatPairStatsUsdOhlc`, `twapRawToDecimalString` |
| `frontend-dapp/src/utils/chartsOverviewStats.ts` | `formatIndexedVolumeUsd` |
| `frontend-dapp/src/utils/limitOrderPriceScale.ts` | Human scale `10^(d0−d1)` |
| `frontend-dapp/src/pages/ChartsPage.tsx` | 24h Stats layout + TWAP StatBoxes |
| `frontend-dapp/src/components/charts/priceChartCandles.ts` | Histogram human scale |
| `frontend-dapp/src/components/charts/PriceChart.tsx` | Passes pair-leg decimals into histogram |

## Regression

```bash
make verify-issue-565
make verify-issue-564
```

Vitest: `chartsPairStats.test.ts`, `ChartsPage.test.tsx` (#565 + #564 blocks), `priceChartCandles.test.ts`, `oracle.test.ts`.

## Related

- [`AGENTS_FRONTEND_CHARTS_OVERVIEW.md`](./AGENTS_FRONTEND_CHARTS_OVERVIEW.md) — overview 24h USD (#548)
- [`AGENTS_FRONTEND_TRADER_VOLUME_USD.md`](./AGENTS_FRONTEND_TRADER_VOLUME_USD.md) — leaderboard / profile USD (#553)
- [`AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](./AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) — picker/pool badges ≠ pair Stats (#534 / #544)
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — P522-Q catalog + factory `*_usd` OHLC (#522)
- [`AGENTS_FRONTEND_USD_CANDLE_INVERT.md`](./AGENTS_FRONTEND_USD_CANDLE_INVERT.md) — volume is not inverted as price (#543)
- [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) — invert does not reassign stats (#524)
- [`AGENTS_FRONTEND_PRICE_CHART.md`](./AGENTS_FRONTEND_PRICE_CHART.md) — histogram wiring
- [`AGENTS_LIMIT_PRICE_DECIMALS.md`](./AGENTS_LIMIT_PRICE_DECIMALS.md) — same `10^(d0−d1)` scale as TWAP (#529)
