# Agent playbook: Protocol top-5 30d pair volume (Forgejo #1263)

Audience: third-party agents changing `/protocol` ranking, `GET /api/v1/protocol/top-pairs`, or `pair_volume_30d`.

**Issue:** [#1263](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1263)  
**UI invariants:** [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) (**P1263-1–P1263-8**)  
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
