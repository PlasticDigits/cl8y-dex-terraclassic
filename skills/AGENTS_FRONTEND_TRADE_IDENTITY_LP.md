# Agent playbook: `/trade` + `/charts` v2 LP USD identity chip

Audience: third-party agents adding pool-size chrome on `/trade` or `/charts`, or changing `GET /api/v1/pairs/{addr}`.

**Issue:** [GitLab **#664**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/664)  
**Invariants:** [`docs/frontend.md` § Token identity](../docs/frontend.md#token-identity) (**T664-1–T664-8**), [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (single-pair `liquidity_usd`)  
**Depends on:** [#569](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/569) `protocol_pair_tvl` catalog. Shares the `pair_liquidity_usd` stamp with [#655](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/655) (`/pool` column + list JOIN — **not** this ticket).

## Problem class

Traders on `/trade/{pair}` see *which* tokens, not *how large* the factory v2 pool is. Charts 24h **Vol (USD)** is **flow**, not stock. The census number already exists in-process (`protocol_pair_tvl`). Do not LCD `getPool`, invent USD from volume, or fork a second catalog.

## Do / don’t

- **Do** read `liquidity_usd` from existing `GET /api/v1/pairs/{addr}` (`getPair`). Trade already fetches it (`indexer-pair-trade`).
- **Do** pass it into [`PairTokenLinks`](../frontend-dapp/src/components/ui/PairTokenLinks.tsx) as `liquidityUsd` on Trade and Charts only.
- **Do** format with [`formatPairV2LpUsd`](../frontend-dapp/src/utils/formatProtocolStats.ts) (plain decimal → `formatProtocolUsd`; hostile / scientific / `≤0` → omit).
- **Do** stamp `pair_liquidity_usd` on protocol TVL refresh — **not** on GET.
- **Don’t** pass `liquidityUsd` on `/pool` table `PairTokenLinks` (column is #655).
- **Don’t** add a Charts 24h Stats “TVL” tile (nests chrome; collides with Vol USD).
- **Don’t** `getPool` / hub / overview on Trade first paint. **Don’t** live-sum `pair_reserves` × oracles in `get_pair`.
- **Don’t** put TVL on `/stats` (`volume_usd` stays 24h notional).
- **Don’t** `$1` UST1, `2.5×` USTR, vFDUSD conversion, book escrow, or wallet LP share.
- **Don’t** treat unpriced as `$0`. Omit the chip (or em-dash). Lecture copy is forbidden (#489).

## Invariants

| ID | Meaning |
|----|---------|
| **T664-1** | `/trade/{pair}` identity shows **v2 LP** + compact `$` when `liquidity_usd` is priced. |
| **T664-2** | `/charts/{pair}` identity shows the same chip. Charts 24h **Vol (USD)** stays `volume_usd`. |
| **T664-3** | `GET /api/v1/pairs/{addr}` optional `liquidity_usd` from the stamp; omitted/`null` when unpriced; unknown addr **404**. |
| **T664-4** | Value is `protocol_pair_tvl` (P522-Q + hub). Never `$1` UST1, never `2.5×` USTR, never vFDUSD. |
| **T664-5** | Unpriced / `""` / `"Infinity"` / `"NaN"` / hostile → omit chip; never fake `$0`. |
| **T664-6** | #524 invert does not change USD or identity payloads (**T541-5**). |
| **T664-7** | Invalid / unknown pair: no identity, no LP chip. Pair switch must not flash the previous pair’s `$`. |
| **T664-8** | `/pool` table identity does **not** take `liquidityUsd`. |

## Canonical code

| File | Role |
|------|------|
| `indexer/migrations/20260826150000_pair_liquidity_usd.sql` | Stamp table (no `$0` row for unpriced) |
| `indexer/src/indexer/protocol_tvl.rs` | Refresh writes stamps via `collect_priced_pair_tvls` |
| `indexer/src/db/queries/pair_liquidity_usd.rs` | Replace-all + GET-by-`pair_id` |
| `indexer/src/api/pairs.rs` | `PairResponse.liquidity_usd` on single GET only |
| `frontend-dapp/src/components/ui/PairTokenLinks.tsx` | Optional compact **v2 LP** chip |
| `frontend-dapp/src/pages/TradePage.tsx` / `ChartsPage.tsx` | Pass `getPair` field; key on pair address |
| `frontend-dapp/src/utils/formatProtocolStats.ts` | `formatPairV2LpUsd` |

## Regression

```bash
make verify-issue-664
```

Indexer: `GET /pairs/{addr}` stamp / omit / 404. Vitest: `PairTokenLinks` + Trade + Charts `#664`. Playwright: `e2e/trade-identity-lp-664.spec.ts` (5 workers, no e2e-tx). `make verify-issue-541` must stay green. `PairTokenLinks` / `TokenIdentity` tests mock `getExplorerAddressUrl` via `importOriginal` so AddressRow keeps real `isSafeExplorerHref` (#430). Do not stub the whole `terraExplorer` module.

## Related

- [`AGENTS_FRONTEND_TOKEN_IDENTITY.md`](./AGENTS_FRONTEND_TOKEN_IDENTITY.md) — chips / copy / explorer (**T541**)
- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) — **P569** catalog (do not fork)
- [`AGENTS_FRONTEND_POOL_TABLE.md`](./AGENTS_FRONTEND_POOL_TABLE.md) — `/pool` column is **#655**, not this chip
- [`AGENTS_FRONTEND_CHARTS_PAIR_STATS.md`](./AGENTS_FRONTEND_CHARTS_PAIR_STATS.md) — 24h Vol USD is flow (**P565-1**)
- [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) — no `StatBox` / `card-glass` in the pair-select panel
- [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) — hub marks for UST1/USTR
- [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) — invert must not change USD
- Post-merge leftover: [#673](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/673) / `make verify-issue-673` / [`AGENTS_POST_MERGE_OPS_673.md`](./AGENTS_POST_MERGE_OPS_673.md)
