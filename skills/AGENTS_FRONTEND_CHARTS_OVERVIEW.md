# Agent playbook: Charts overview 24h volume USD-only (GitLab #548)

Audience: third-party agents touching `/charts` overview stats, `GET /api/v1/overview`, or indexer `swap_events.volume_usd`.

**Issue:** [GitLab **#548**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548)  
**Invariants:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (rows **Overview global 24h stats**, **Charts overview USD #548**, **External oracle tickers #515** **X4**)  
**Frontend:** [`docs/frontend.md`](../docs/frontend.md) § Charts overview strip

## Problem class

Charts showed a raw mixed-decimal **24h Volume** (`10,000,000T` from `SUM(offer_amount)`) next to **24h Volume (USD) = 0**. Ingest `compute_volume_usd` was USTC-leg only with a hardcoded `1e6` decimals factor, so UST1/USTR (and other P522-Q legs) stored `volume_usd` NULL. Retail must see **one** 24h volume figure, in **USD**.

Pair-search / `/pool` USD badges stay on [#544](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/544). Pair-level 24h **Vol (USD)** + human token remainder is [#565](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/565) ([`AGENTS_FRONTEND_CHARTS_PAIR_STATS.md`](./AGENTS_FRONTEND_CHARTS_PAIR_STATS.md)). Trader leaderboard / profile lifetime volume is [#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553). **Share** [`volume_usd_for_swap`](../indexer/src/indexer/pair_price_usd.rs) — do not fork a second USD formula.

## Invariants (C1–C9)

| ID | Rule |
|----|------|
| **C1** | `/charts` overview has **exactly one** 24h volume control, in USD (`$` + compact human). No raw `24h Volume` box. Never pass `total_volume_24h` to `formatNum`. |
| **C2** | Catalog-priced 24h swaps (UST1/USTR, UST1/cUSTC, …) → positive compact USD matching `GET /api/v1/overview` `total_volume_24h_usd` (rollup lag ≤ ~5 min). |
| **C3** | Unpriced activity (unknown quote / oracle down) → **`—`**, not `$0`. Idle DEX (`total_trades_24h === 0`) → **`$0`**. API sends JSON `null` (not `"0"`) when trades > 0 and rollup USD is 0. |
| **C4** | **USTC / USD** = `$` + `formatPairPrice` of overview `ustc_price_usd`. Null/invalid → `—`. Never LUNC. Never compact `T`. |
| **C5** | **24h Trades** = 24h `swap_events` count (**L10**). Do not add `limit_order_fills`. |
| **C6** | **Pairs** = `COUNT(*) FROM pairs`. **Tokens** = unique pair-leg assets (`COUNT` SQL over `asset_0_id` ∪ `asset_1_id`). Not `get_all_assets().len()`, not LP tokens. |
| **C7** | JSON keeps `total_volume_24h` (raw `SUM(offer_amount)`) for integrators. Charts does not display it. |
| **C8** | Ingest uses P522-Q once (`quote_usd_kind_for_asset` + `usd_per_human_quote`). Humanize with per-asset decimals. Unknown → NULL. |
| **C9** | Outage banner (#215) still hides/skeletons the strip; no `VITE_INDEXER_URL` leak. |

## Ingest (shared with #544)

Prefer catalog-known **pair quote** (`asset_1`): human quote amount × quote USD. Else offer, else ask. **Never sum both legs.** Classify factory asset rows (denom `uusd`/`uluna`, CW20 hub tickers with a contract). Native gems that spoof `symbol=USTR` must not price (**A1**). Factory-listed CW20 clones with hub tickers remain a residual — provenance (#311) is the pair-insert gate.

Backfill: [`volume::backfill_swap_volume_usd`](../indexer/src/db/queries/volume.rs) + migration `20260817120000_backfill_swap_volume_usd_catalog.sql`. Idempotent. USD is as-of ingest/backfill oracles (advisory, **X5**). Then `refresh_global_stats`.

**X4 (updated):** `volume_usd` uses the **P522-Q catalog** (USTC/cUSTC/`uusd`=#515 USTC; LUNC/cLUNC/`uluna`=#515 LUNC; **UST1/USTR = hub_prices**, [#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556)). Overview **USTC / USD** box stays the USTC ticker only.

## Do / don’t

- **Do** format overview USD with [`formatChartsOverviewVolumeUsd`](../frontend-dapp/src/utils/chartsOverviewStats.ts).
- **Do** keep CG/CMC units and candle histogram unchanged.
- **Don’t** invent USD for faucet gems.
- **Don’t** use overview USD for settlement/fees.
- **Don’t** load the full `assets` table on `/overview`.
- **Don’t** implement a second `compute_volume_usd` for #544 — call `volume_usd_for_swap`.

## Regression checklist

1. `cd indexer && cargo test --lib pair_price -- --quiet`
2. `cd indexer && cargo test --test volume_usd_catalog --test api_overview --test indexer_overview_global_stats -- --test-threads=1`
3. Frontend: `chartsOverviewStats.test.ts`, `ChartsPage.test.tsx` overview strip
4. `make verify-issue-548`

## Related

- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — P522-Q catalog
- [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md) — USTC/LUNC feeds
- [`AGENTS_INDEXER_VOLUME_PAGINATION.md`](./AGENTS_INDEXER_VOLUME_PAGINATION.md) — rollup + 60s cache (**V5**)
- [`AGENTS_FRONTEND_CHARTS_PAIR_STATS.md`](./AGENTS_FRONTEND_CHARTS_PAIR_STATS.md) — pair-detail 24h Vol (USD) (#565)
- [`AGENTS_FRONTEND_TRAILING_WINDOW.md`](./AGENTS_FRONTEND_TRAILING_WINDOW.md) — trailing 24h copy, not midnight reset (#576)
- [`AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](./AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) — pair-list volume badges (#534 / #544)
- [`AGENTS_FRONTEND_CHARTS_PAIR_STATS.md`](./AGENTS_FRONTEND_CHARTS_PAIR_STATS.md) — Charts pair 24h Stats **Vol (token)** / TWAP human scale (#564); overview stays USD-only
- [`AGENTS_FRONTEND_TRADER_VOLUME_USD.md`](./AGENTS_FRONTEND_TRADER_VOLUME_USD.md) — Charts leaderboard + trader profile USD (#553)
- [`AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](./AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md) — #215 banner
- [`docs/runbooks/overview-global-stats-brin.md`](../docs/runbooks/overview-global-stats-brin.md)
