# Agent playbook: DeFiLlama listing (GitLab #631)

Audience: third-party agents changing `GET /api/v1/defillama/daily`, the UTC-day rollup, gem exclude, or the vendored Llama adapters under `scripts/defillama/`.

**Issue:** [GitLab **#631**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631)  
**Playbook:** [`docs/DEFILLAMA.md`](../docs/DEFILLAMA.md)  
**Invariants table:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **DeFiLlama UTC-day #631**)  
**Hybrid volume:** [`docs/integrators-hybrid-volume.md`](../docs/integrators-hybrid-volume.md) **L10** / [`AGENTS_INTEGRATOR_HYBRID_VOLUME.md`](./AGENTS_INTEGRATOR_HYBRID_VOLUME.md)  
**Fees:** [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) PFee / [`AGENTS_INDEXER_WRAP_FEE_INGEST.md`](./AGENTS_INDEXER_WRAP_FEE_INGEST.md)  
**Gems:** [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) **#562**  
**TVL drift:** [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) **P569**

This is **not** a CoinGecko/CMC ticket ([`docs/CG_CMC_COMPLIANCE.md`](../docs/CG_CMC_COMPLIANCE.md), [#224](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/224)).

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
| **L631-7** | GET `/api/v1/defillama/daily` is O(1) rollup + 60s cache. Invalid timestamp → **400**. Missing day → **404**. Idle → `"0"`. Activity + unpriced → `null`. Single-day param only. |
| **L631-8** | No `misrepresentedTokens` pegs unless Llama + methodology name the substitution. vFDUSD / CEX FDUSD is not a pool asset. |
| **L631-9** | This skill + `docs/DEFILLAMA.md` + `make verify-issue-631`. Keep `make verify-issue-586` / `569` / `562`. |

## Do / don’t

- **Do** refresh the daily rollup from the existing ~5 min volume loop, not on GET.
- **Do** keep gem addresses in lockstep: frontend `#562` set, `indexer/src/indexer/defillama.rs`, `scripts/defillama/gems.js`.
- **Do** pin the dimension adapter host to `https://indexer.dex.cl8y.com` (A18).
- **Don’t** publish CG `liquidity_in_usd` as Llama TVL (it is mislabeled 24h volume).
- **Don’t** bind-mount `indexer/` into root Docker for cargo (`make test-indexer-target-ownership`).
- **Don’t** treat `overview.total_volume_24h_usd` as `dailyVolume`.
- **Don’t** open Llama GitHub PRs from this repo’s CI — copies live under `scripts/defillama/`.

## Key files

| Area | Path |
|------|------|
| Daily GET | [`indexer/src/api/defillama.rs`](../indexer/src/api/defillama.rs) |
| Rollup | [`indexer/src/db/queries/defillama.rs`](../indexer/src/db/queries/defillama.rs) |
| Gems + parse | [`indexer/src/indexer/defillama.rs`](../indexer/src/indexer/defillama.rs) |
| Migration | [`indexer/migrations/20260825140000_defillama_daily.sql`](../indexer/migrations/20260825140000_defillama_daily.sql) |
| TVL helper | [`scripts/defillama/tvl/tvlCore.js`](../scripts/defillama/tvl/tvlCore.js) |
| Tests | [`indexer/tests/indexer_defillama.rs`](../indexer/tests/indexer_defillama.rs) |

## Regression

```bash
make setup-indexer-postgres
make verify-issue-631
```

Live Coolify (operator, after deploy): yesterday UTC `GET https://indexer.dex.cl8y.com/api/v1/defillama/daily?timestamp=<00:00_utc>`.

## Related

- [`docs/DEFILLAMA.md`](../docs/DEFILLAMA.md)
- [`AGENTS_INTEGRATOR_HYBRID_VOLUME.md`](./AGENTS_INTEGRATOR_HYBRID_VOLUME.md)
- [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md)
- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md)
- [#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629) Llama pricing coverage
