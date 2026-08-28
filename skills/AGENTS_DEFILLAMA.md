# Agent playbook: DeFiLlama listing (GitLab #631)

Audience: third-party agents changing `GET /api/v1/defillama/daily`, the UTC-day rollup, gem exclude, or the vendored Llama adapters under `scripts/defillama/`.

**Issue:** [GitLab **#631**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631) (listing + daily GET); leftover [GitLab **#687**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/687) (headline partial SUM + adapter start / 404)  
**Playbook:** [`docs/DEFILLAMA.md`](../docs/DEFILLAMA.md)  
**Invariants table:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **DeFiLlama UTC-day #631**)  
**Hybrid volume:** [`docs/integrators-hybrid-volume.md`](../docs/integrators-hybrid-volume.md) **L10** / [`AGENTS_INTEGRATOR_HYBRID_VOLUME.md`](./AGENTS_INTEGRATOR_HYBRID_VOLUME.md)  
**Fees:** [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) PFee / [`AGENTS_INDEXER_WRAP_FEE_INGEST.md`](./AGENTS_INDEXER_WRAP_FEE_INGEST.md)  
**Gems:** [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) **#562**  
**TVL drift:** [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) **P569**

This is **not** a CoinGecko/CMC ticket ([`docs/CG_CMC_COMPLIANCE.md`](../docs/CG_CMC_COMPLIANCE.md), [#224](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/224) / [#685](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/685)).

## Problem class

Llama lists Terra Classic DEXes via **their** adapter repos. CL8Y `/cg/*`, `/cmc/*`, and trailing `/overview` windows are the wrong shape (24h snapshot, indexer USD). #631 adds a UTC-day rollup + public GET, vendors adapter copies, and documents methodology.

## Invariants (L631)

| ID | Rule |
|----|------|
| **L631-1** | TVL adapter `api.add`s factory `Pool {}` raw denom/CW20 only. Never indexer USD, hub marks, or CG `liquidity_in_usd`. |
| **L631-2** | `timetravel: false`. Paginate `Pairs` until a short page. Fail if pool-query errors `> count/2`. Do not add LP `liquidity_token`. |
| **L631-3** | Volume is UTC calendar day (`timestamp % 86400 == 0`), `SUM(swap_events.volume_usd)` once per row (**L10**). Never add `limit_order_fills`. |
| **L631-4** | Exclude columbus-5 gem pairs from daily volume and pair-linked fees. Same address set as `COLUMBUS5_GEM_ADDRESSES`. |
| **L631-5** | Exclude wrap/unwrap and UST1 mint/redeem from `dailyVolume`. They may appear as **labeled** fee breakdowns. |
| **L631-6** | Fees are PFee/L7 treasury sources. `spread_amount`, burn tax, gas, hooks, and community-tax extra-debit are not `dailyFees`. SSR is `0`. |
| **L631-7** | GET `/api/v1/defillama/daily` is O(1) rollup + 60s cache. Invalid timestamp → **400**. Missing day → **404**. Idle → `"0"`. Volume: activity + unpriced → `null`. Headline **fees** are EFee-6 partial SUM (**L687**); per-source `fees.*` stay fail-closed. Single-day param only. |
| **L631-8** | Named wrap substitution only: cLUNC → `uluna`, cUSTC → `uusd` (1:1). No UST1=$1 / USTR hub pegs. vFDUSD / CEX FDUSD is not a pool asset. |
| **L631-9** | This skill + `docs/DEFILLAMA.md` + `make verify-issue-631`. Keep `make verify-issue-586` / `569` / `562`. Headline null leftover: `make verify-issue-687`. |
| **L631-10** | UST1 is the **unstablecoin** on Llama Stablecoins (`peggedUSD`, crypto-backed). Circulating is CW20 `total_supply`. Price is hub / Llama — never `$1`. |
| **L631-11** | USTR is reserve-token info on `assets.ustr` (volume, pair fees, hub price). Not a second stablecoin. Not 2.5× USTC. |

## Headline fees leftover (L687 / GitLab #687)

Llama [dimension-adapters#8987](https://github.com/DefiLlama/dimension-adapters/pull/8987) throws when `daily_fees_usd` is JSON `null`, and axios throws on HTTP **404**. The Coolify route is **live**. Do **not** reopen [#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631) or [#683](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/683).

| ID | Rule |
|----|------|
| **L687-1** | Headline `daily_fees_usd` / `daily_revenue_usd` / `daily_protocol_revenue_usd` = **priced SUM** (EFee-6). One unpriced source must **not** null wrap/window/hub. |
| **L687-2** | Headline `null` only when fee **activity exists** and priced SUM is empty. Idle (no fee events) is `"0"` even if volume is non-zero. Never map unpriced → `"0"`. |
| **L687-3** | Per-source `fees.swap_amm` (etc.) stay fail-closed. SSR is `"0"`. Revenue equals fees. Residual in the adapter is unlabeled USD, **not** `dailySupplySideRevenue`. |
| **L687-4** | Adapter `start` is the first UTC day GET returns **200**. Pin: unix `1786924800` / ISO `2026-08-17`. `start - 86400` is **404**. Do not move earlier. Do not swallow 404 as `$0`. |
| **L687-5** | Adapter throws on JSON `null` (all-unpriced) and HTTP 5xx / malformed JSON. Does **not** throw on `"0"`. Version stays **1**. |
| **L687-6** | Fees adapter adds Llama METRIC groups + residual only. Never `addUSDValue(total)` then add breakdown labels again. |
| **L687-7** | GET stays O(1) rollup + 60s cache. No `from`/`to`. Refresh lookback stays **8** UTC days. Optional operator backfill of `defillama_daily_*` from `start` is OK; not a live GET scan. |
| **L687-8** | `make verify-issue-687`. Keep `make verify-issue-631` and `make verify-issue-683` green. Patch GitHub #8987 in lockstep with `scripts/defillama/` (operator; not CI). |

## Do / don’t

- **Do** refresh the daily rollup from the existing ~5 min volume loop, not on GET.
- **Do** keep gem addresses in lockstep: frontend `#562` set, `indexer/src/indexer/defillama.rs`, `scripts/defillama/gems.js`.
- **Do** pin the dimension adapter host to `https://indexer.dex.cl8y.com` (A18).
- **Do** pin dimension adapter `start` to the first **200** UTC day (`ADAPTER_START` / `ADAPTER_START_ISO` in `scripts/defillama/gems.js`).
- **Don’t** OR `unpriced_count` across fee sources into a whole-day headline `null` while wrap/window are priced.
- **Don’t** publish CG `liquidity_in_usd` as Llama TVL (Llama is on-chain `Pool {}` only; CG is indexer USD stamp — [#685](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/685)).
- **Don’t** bind-mount `indexer/` into root Docker for cargo (`make test-indexer-target-ownership`).
- **Don’t** treat `overview.total_volume_24h_usd` as `dailyVolume`.
- **Don’t** list USTR as `peggedUSD` or invent a UST1 `$1` peg for Llama.
- **Don’t** open Llama GitHub PRs from this repo’s CI — copies live under `scripts/defillama/`. Filed: [DefiLlama-Adapters#20676](https://github.com/DefiLlama/DefiLlama-Adapters/pull/20676), [dimension-adapters#8987](https://github.com/DefiLlama/dimension-adapters/pull/8987), [peggedassets-server#903](https://github.com/DefiLlama/peggedassets-server/pull/903).

## Key files

| Area | Path |
|------|------|
| Daily GET | [`indexer/src/api/defillama.rs`](../indexer/src/api/defillama.rs) |
| Rollup | [`indexer/src/db/queries/defillama.rs`](../indexer/src/db/queries/defillama.rs) |
| Gems + parse | [`indexer/src/indexer/defillama.rs`](../indexer/src/indexer/defillama.rs) |
| Migration | [`indexer/migrations/20260825150000_defillama_daily.sql`](../indexer/migrations/20260825150000_defillama_daily.sql) |
| Asset rollup | [`indexer/src/db/queries/defillama_assets.rs`](../indexer/src/db/queries/defillama_assets.rs) + [`20260825160000_defillama_daily_assets.sql`](../indexer/migrations/20260825160000_defillama_daily_assets.sql) |
| UST1 pegged copy | [`scripts/defillama/stablecoins/`](../scripts/defillama/stablecoins/) |
| TVL helper | [`scripts/defillama/tvl/tvlCore.js`](../scripts/defillama/tvl/tvlCore.js) |
| Tests | [`indexer/tests/indexer_defillama.rs`](../indexer/tests/indexer_defillama.rs) |
| Mapper | [`scripts/defillama/dimensions/mapDaily.js`](../scripts/defillama/dimensions/mapDaily.js) |

## Regression

```bash
make setup-indexer-postgres
make verify-issue-631
make verify-issue-687
```

Live Coolify (operator, after deploy): yesterday UTC `GET https://indexer.dex.cl8y.com/api/v1/defillama/daily?timestamp=<00:00_utc>`. Llama `pnpm test fees cl8y-dex` / `pnpm test dexs cl8y-dex` (yesterday + `start`) is operator follow-up on [dimension-adapters#8987](https://github.com/DefiLlama/dimension-adapters/pull/8987).

## Related

- [`docs/DEFILLAMA.md`](../docs/DEFILLAMA.md)
- [`AGENTS_INTEGRATOR_HYBRID_VOLUME.md`](./AGENTS_INTEGRATOR_HYBRID_VOLUME.md)
- [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md)
- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md)
- [`AGENTS_INDEXER_ECONOMIC_FEE_USD.md`](./AGENTS_INDEXER_ECONOMIC_FEE_USD.md) — daily fees inherit stamped CL8Y / economic `fee_usd` ([#683](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/683)); headline partial SUM ([#687](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/687))
- [#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629) Llama pricing coverage
- [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639) other listing venues — [`AGENTS_LISTINGS.md`](./AGENTS_LISTINGS.md)
- [#687](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/687) Llama fees null/404 leftover — `make verify-issue-687`
