# Agent playbook: Protocol top-5 30d pair volume (Forgejo #1263)

Audience: third-party agents changing `/protocol` ranking, `GET /api/v1/protocol/top-pairs`, or `pair_volume_30d`.

**Issues:** [#1263](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1263) (shipped baseline) · [#1317](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1317) (open CMM-held LP extension)<br>
**UI invariants:** [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) (**P1263-1–P1263-8**)  
**System invariant table:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (I1263 + pending I1317)<br>
**24h pair list (not this):** [`AGENTS_INDEXER_PAIR_VOLUME_USD.md`](./AGENTS_INDEXER_PAIR_VOLUME_USD.md) (**PVol** — `GET /pairs` `volume_usd_24h`)  
**Current TVL stamp:** [`AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md`](./AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md) (**P655**)  
**Gem exclude:** [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) (**P562**) / [`AGENTS_DEFILLAMA.md`](./AGENTS_DEFILLAMA.md) (`COLUMBUS5_GEM_ADDRESSES`)

## Problem class

Global 30d volume and total TVL cannot rank **which pool** is earning its keep. `/pool` has 24h Vol + v2 LP only. This GET is a **hard-capped census of five** factory pairs by trailing 30d priced USD, with **current** AMM TVL and a server-side vol/LP ratio.

## Invariants (I1263)

| ID | Rule |
|----|------|
| **I1263-1** | Stamp trailing 30d pair USD in `pair_volume_30d` inside `refresh_pair_volumes_30d` (~5 min + startup). Same NULL / ≥10^20 / idle→0 rules as #692. Do not COALESCE unpriced to 0. |
| **I1263-2** | `GET /api/v1/protocol/top-pairs` JOINs `pair_volume_30d` + `pair_liquidity_usd`. **EXPLAIN** must not mention `swap_events` or `pair_reserves`. |
| **I1263-3** | Rank priced `volume_usd > 0` only, `ORDER BY volume_usd DESC NULLS LAST, pair_id ASC LIMIT 5`. Idle 0 does not fill empty slots. Cold → `items: []`. |
| **I1263-4** | Exclude pairs whose either CW20 leg is in `COLUMBUS5_GEM_ADDRESSES` (`gem_addresses_lowercased`). Do not fork the list. Address wins over spoof tickers. |
| **I1263-5** | `volume_per_tvl` = `volume_usd_30d / liquidity_usd` in the indexer. TVL missing / `≤0` / ≥10^20 → JSON omit/`null`. Never Inf / NaN / `0` as a stand-in. |
| **I1263-6** | Query allowlist: `limit` omitted or `5`; `window` omitted or `30d`. `limit=1`/`6`, `window=24h`, `from`, `to`, `sort`, `ticker` → **400** (do not truncate). 60s cache is a single slot; junk keys ignored. |
| **I1263-7** | Do **not** add `sort=volume_usd_30d` to `GET /pairs`. Do **not** change `GET /overview` shape. Do **not** N+1 `/stats`. |
| **I1263-8** | Same USD catalog as #548 / #556 / #569. Never vFDUSD, never `$1` UST1, never `2.5×` USTR, never CG `liquidity_in_usd`. |

## Pending extension: CMM-held LP census (#1317)

The current API, SQL, response type, and table implement **#1263 only**. The open #1317 acceptance criteria are not met on `origin/main`; do not imply that the following fields or columns exist, and do not close #1317 based on `make verify-issue-1263`.

| ID | Required invariant before #1317 can close |
|----|--------------------------------------------|
| **I1317-1** | Preserve #1263 top-five membership/order, gem exclusion, full-pool `liquidity_usd`, and `volume_per_tvl`; the CMM metrics do not rerank rows or redefine Vol/LP. |
| **I1317-2** | `cmm_lp_usd_30d` is the trailing-30d time-weighted USD of factory LP shares actually held by the configured CMM custodian. Resolve bare LP-token transfers or seed the integral from an off-request balance observation. Never substitute spot TVL or “pool minus CMM.” |
| **I1317-3** | `trading_fees_usd_30d` includes only priced pair `swap_amm`, `book_take`, and `limit_place` fees; wrap/unwrap and UST1 mint/redeem fees are excluded. Unpriced activity is `NULL`, not zero; idle is zero. |
| **I1317-4** | Refresh volume, fee, and LP-seconds stamps off-request. `GET /api/v1/protocol/top-pairs` reads stamps only: no `swap_events`, `protocol_fee_events`, `liquidity_events` scan, or LCD fanout. Extend the 60s response cache for the additive fields. |
| **I1317-5** | `fees_bps_per_cmm_lp = trading_fees_usd_30d / cmm_lp_usd_30d * 10_000`; `volume_per_cmm_lp = volume_usd_30d / cmm_lp_usd_30d`. Missing, non-positive, or ≥10^20 denominator/ratio serializes `null`; UI shows an em dash. No APR, farm, points, fee split, or `FEE_CONFIG` change. |

**Implementation map:** [`protocol_top_pairs.rs`](../indexer/src/api/protocol_top_pairs.rs), [ranking SQL](../indexer/src/db/queries/protocol_top_pairs.rs), [`protocol_fees.rs`](../indexer/src/db/queries/protocol_fees.rs), [`liquidity.rs`](../indexer/src/db/queries/liquidity.rs), [`ProtocolTopPairs.tsx`](../frontend-dapp/src/components/protocol/ProtocolTopPairs.tsx), and [`docs/frontend.md` § Protocol](../docs/frontend.md#protocol-page). Related issues: [#1263](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1263) baseline, [#1269](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269) complete multihop fee ingestion, [#558](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/558) custodian LP operations, and [#1207](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1207) separate yield research.

Before closing #1317, add indexer fixtures for the LP-seconds integral, transfer-gap choice, fee-source filter, null/overflow rules, and GET query plan/cache; add API/OpenAPI and frontend column/copy/null/four-old-columns regressions. Keep the #1263 verifier as a baseline, not a substitute.

## Do / don’t

- **Do** call `refresh_pair_volumes_30d` from `refresh_all_volume_windows_with_pins` next to the 24h pair stamp.
- **Do** reuse `COLUMBUS5_GEM_ADDRESSES` / `gem_addresses_lowercased`.
- **Don’t** live-SUM 30d `swap_events` on GET.
- **Don’t** rank gems or unpriced NULL volume.
- **Don’t** compute the ratio in the browser from `/pairs?sort=volume_usd_24h&limit=5`.
- **Don’t** add BTC/ETH/stock rows, farm/APR chrome, or a Protocol bar chart of the same five pairs.

## Verify

```bash
make verify-issue-1263
```

Keep `make verify-issue-550` / `569` / `655` / `692` / `653` / `562` green.
