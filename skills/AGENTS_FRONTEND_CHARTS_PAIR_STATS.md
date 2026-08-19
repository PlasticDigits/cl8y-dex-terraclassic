# Agent playbook: Charts pair 24h Vol (USD) + human token remainder (GitLab #565)

Audience: third-party agents touching `/charts` **pair 24h Stats**, `GET /api/v1/pairs/{addr}/stats`, or `IndexerPairStats`.

**Issue:** [GitLab **#565**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/565)  
**Invariants:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Charts pair 24h volume #565**), [`docs/frontend.md`](../docs/frontend.md) § Charts pair 24h stats  
**Frontend:** [`ChartsPage.tsx`](../frontend-dapp/src/pages/ChartsPage.tsx) 24h Stats strip

## Problem class

Same `formatNum(raw)` bug as tape Amount ([#557](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557)), picker volume ([#534](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534)), and overview USD ([#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548)). Pair-level **Vol (token)** was left on production after #540 / #544 **AC4**: UST1/cUSTC showed **847.0M / 157.5B** for ~847 UST1 / ~157K cUSTC. UST1/USTR 18-dec quote compact-formatted as **`T`**.

Indexer already returns human `volume_usd` and raw `volume_base` / `volume_quote`. Do **not** humanize those JSON fields.

## Invariants (P565-1–P565-7)

| ID | Rule |
|----|------|
| **P565-1** | Pair 24h stats **primary** volume is **Vol (USD)** = [`formatIndexedVolumeUsd`](../frontend-dapp/src/utils/chartsOverviewStats.ts)`(stats.volume_usd, stats.trade_count)`. `$` + compact. `data-testid="charts-pair-volume-usd"`. Tooltip: `24h volume in USD`. Advisory, not a peg (**X5** / P522). |
| **P565-2** | Never `formatNum(stats.volume_base)` or `formatNum(stats.volume_quote)`. UST1/USTR raw 18-dec quote must not print `T`. |
| **P565-3** | Optional secondary **Vol ({symbol})** uses [`formatChartsPairTokenVolume`](../frontend-dapp/src/utils/chartsPairStats.ts) with **that pair’s** `asset_0` / `asset_1` decimals (factory order). Missing / non-integer / out-of-range decimals → `—`. Never assume 6. Never match decimals by symbol. |
| **P565-4** | Unpriced / invalid / negative `volume_usd` with `trade_count > 0` → `—`, not `$0`, not raw fallback. Idle (`trade_count === 0` and USD `0`) → `$0` (same contract as #548 **C3**). |
| **P565-5** | [#524](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524) invert does **not** change the USD figure and does **not** swap base/quote raw onto the other leg’s decimals. Pair volume is pair-level. |
| **P565-6** | Integrator JSON keeps raw `volume_base` / `volume_quote`. `volume_usd` stays human USD. CG/CMC and candle histogram unchanged. One notional (**L10**) — display `stats.volume_usd` only. |
| **P565-7** | Token vols render only when `activePair.pair_address` is the selected pair that fetched stats. Hostile `volume_usd` / symbol is text-only (no `dangerouslySetInnerHTML`). |

## Do / don’t

- **Do** reuse `formatIndexedVolumeUsd`. Do not invent a third USD formatter. Do not divide already-human `volume_usd` by 1e6.
- **Do** keep secondary token boxes on a second row so USD is the default read.
- **Don’t** implement tape Amount in/out (#557), overview strip (#548), or picker/pool badges (#544) here.
- **Don’t** add candle volumes, limit fills, or pool+book legs into the USD box.
- **Don’t** convert DEX volume with vFDUSD (**X4**).

## Regression checklist

1. Frontend: `chartsPairStats.test.ts`, `ChartsPage.test.tsx` pair 24h stats
2. Grep: `ChartsPage.tsx` has no `formatNum(stats.volume_base)` / `formatNum(stats.volume_quote)`
3. `make verify-issue-565`

## Related

- [`AGENTS_FRONTEND_CHARTS_OVERVIEW.md`](./AGENTS_FRONTEND_CHARTS_OVERVIEW.md) — overview 24h USD (#548)
- [`AGENTS_FRONTEND_TRADER_VOLUME_USD.md`](./AGENTS_FRONTEND_TRADER_VOLUME_USD.md) — leaderboard / profile USD (#553)
- [`AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](./AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) — picker/pool quote badges (#534 / #544)
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — P522-Q catalog
- [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) — invert does not reassign stats fields (#524)
