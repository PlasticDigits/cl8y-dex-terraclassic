# Agent playbook: Charts trader leaderboard + profile volume USD (GitLab #553)

Audience: third-party agents touching `/charts` **Trader leaderboard**, `/trader/:addr`, `/portfolio` **Total Volume**, or `GET /api/v1/traders/*` volume fields.

**Issue:** [GitLab **#553**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553)  
**Invariants:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Trader volume USD #553**)  
**Frontend:** [`docs/frontend.md`](../docs/frontend.md) § Charts trader leaderboard

## Problem class

Same bug as [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548) / [#534](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534): `formatNum` on raw `SUM(offer_amount)`. USTR 18-dec legs print **`10,000,000T`**. Overview 24h volume is USD-only after #548; the leaderboard **Volume** column and trader **Total Volume** were out of scope there.

Share [`volume_usd_for_swap`](../indexer/src/indexer/pair_price_usd.rs) / `swap_events.volume_usd` — do **not** invent a second USD formula.

## Invariants (T553-1–T553-6)

| ID | Rule |
|----|------|
| **T553-1** | Charts leaderboard **Volume (USD)** is `$` + compact human from `total_volume_usd`. Never pass raw `total_volume` to `formatNum`. Unpriced (`null` / `"0"` with trades) → `—`. Idle (`total_trades === 0`) → `$0`. |
| **T553-2** | [`TraderSummaryStats`](../frontend-dapp/src/components/trader/TraderSummaryStats.tsx) **Total Volume (USD)** uses the same formatter (`formatIndexedVolumeUsd`) on `/trader` and `/portfolio`. |
| **T553-3** | JSON keeps `total_volume` (raw `SUM(offer_amount)`) for integrators. `total_volume_usd` is a decimal string when priced; JSON `null` when `total_trades > 0` and priced USD is 0 (same contract as overview **C3**). |
| **T553-4** | Ingest: `upsert_trader` adds `swap_events.volume_usd` when present. Backfill / catalog re-run: [`refresh_trader_total_volume_usd`](../indexer/src/db/queries/traders.rs) `SUM`s the same column (capped for `NUMERIC(38,18)`). |
| **T553-5** | Charts Volume tab **sorts by `total_volume_usd DESC NULLS LAST`** so the displayed column matches the rank. API still accepts `sort=total_volume` (raw) for integrators; default API sort stays `total_volume`. |
| **T553-6** | Rolling `volume_24h` / `7d` / `30d` stay **raw** API-only. Charts does not show those columns. Realized PnL / fees formatting is out of scope. |

## Do / don’t

- **Do** format with [`formatIndexedVolumeUsd`](../frontend-dapp/src/utils/chartsOverviewStats.ts) (alias of the #548 overview helper).
- **Do** keep CG/CMC and candle histogram unchanged.
- **Don’t** display mixed-unit `total_volume` as retail volume.
- **Don’t** treat `$0` as unpriced activity when `total_trades > 0`.
- **Don’t** convert DEX volume with vFDUSD (**X4**).

## Regression checklist

1. `cd indexer && cargo test --test trader_volume_usd --test api_traders -- --test-threads=1`
2. Frontend: `chartsOverviewStats.test.ts`, `ChartsPage.test.tsx` leaderboard, `TraderSummaryStats.test.tsx`
3. `make verify-issue-553`

## Related

- [`AGENTS_FRONTEND_CHARTS_OVERVIEW.md`](./AGENTS_FRONTEND_CHARTS_OVERVIEW.md) — overview 24h USD (#548)
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — P522-Q catalog
- [`AGENTS_FRONTEND_PORTFOLIO.md`](./AGENTS_FRONTEND_PORTFOLIO.md) — shared `TraderSummaryStats`
- [`AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](./AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) — pair-list volume is still raw quote
