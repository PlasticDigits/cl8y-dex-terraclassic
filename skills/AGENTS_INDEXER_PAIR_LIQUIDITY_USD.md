# Agent playbook: `/pool` v2 LP USD column + pair-list TVL rollup (GitLab #655)

Audience: third-party agents changing `/pool` columns, `GET /api/v1/pairs` JSON, or protocol TVL refresh.

**Issue:** [GitLab **#655**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/655)  
**Invariants:** [`docs/frontend.md` § Liquidity pools list](../docs/frontend.md#liquidity-pools-list-indexer-vs-factory) (**P655-1–P655-8**), [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (pair-list liquidity USD)  
**Parent table:** [`AGENTS_FRONTEND_POOL_TABLE.md`](./AGENTS_FRONTEND_POOL_TABLE.md) (**P547**)  
**TVL math:** [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) (**P569**)

## Problem class

`/pool` showed **flow** (24h quote Vol) but not **stock** (factory AMM TVL). Protocol already computed pair TVL (`protocol_pair_tvl`) for the global sum only. The list API had no field, so the table would cheat with volume, CG `liquidity_in_usd`, or per-row LCD.

## Do / don’t

- **Do** stamp `pair_liquidity_usd` inside `refresh_protocol_liquidity` using `protocol_pair_tvl` (same quotes as #569).
- **Do** JOIN that rollup on `GET /api/v1/pairs`. Sort key `liquidity_usd` (default desc, **NULLS LAST**).
- **Do** render **v2 LP USD** with `formatProtocolUsd` (compact `$` or **—**).
- **Don’t** walk `pair_reserves` + oracles + hub on GET (including `sort=liquidity_usd`).
- **Don’t** invent USD from `volume_quote_24h`, CG `liquidity_in_usd`, LP supply, `$1` UST1, or `2.5×` USTR.
- **Don’t** `getPool` / `getPairFeeConfig` per row on first paint (**P547-9** / A8).
- **Don’t** include book escrow, parked dust, wallet LP share, wrap-mapper inventory, or V3 Grid vaults.
- **Don’t** `COALESCE` unpriced to `0` for display or sort (unpriced ranks last, cell **—**).
- **Don’t** nest `card-glass` per row ([#653](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653)).

## Canonical code

| File | Role |
|------|------|
| `indexer/migrations/20260826150000_pair_liquidity_usd.sql` | Rollup table (`liquidity_usd NOT NULL`; absent row = unpriced) |
| `indexer/src/db/queries/pair_liquidity.rs` | Full replace: DELETE then UNNEST insert of priced pairs |
| `indexer/src/indexer/protocol_tvl.rs` | Shared TVL math + stamp on refresh |
| `indexer/src/db/queries/pairs.rs` | List JOIN + `PairListSort::LiquidityUsd` |
| `indexer/src/api/pairs.rs` | Additive `liquidity_usd` + sort allowlist |
| `frontend-dapp/src/components/pool/PoolPairsTable.tsx` | Column + cell + Manage `colSpan={7}` |
| `frontend-dapp/src/utils/poolListQuery.ts` | `POOL_COLUMN_SORTS` includes `liquidity_usd` |

## Invariants (P655-1–P655-8 / L655)

1. **P655-1** — Column **v2 LP USD** sits immediately after **Vol** (before Fee). Header `title` says factory AMM pool USD (trailing snapshot), not 24h volume.
2. **P655-2** — Cell is `formatProtocolUsd(liquidity_usd)` or **—**. Never `Infinity`, `NaN`, raw 18-dec, or quote-denom volume.
3. **P655-3** — JSON field is optional string when priced; omitted/`null` when not. Additive; `volume_quote_24h` unchanged.
4. **P655-4** — First header click is `sort=liquidity_usd&order=desc`; toggle asc/desc; `aria-sort` on that `<th>` only. Catalog default (no click) still `volume_24h` + client catalog rank (**P547-3**). Search stays indexer `relevance` until the query is cleared.
5. **P655-5** — Stamp = `protocol_pair_tvl` (both legs `h0×usd0+h1×usd1`; one catalogued leg `2×`; never `$1` UST1 / `2.5×` USTR / vFDUSD). Identity is contract/denom.
6. **P655-6** — Unpriced / stale / zero / same-asset / overflow → **no rollup row** (LEFT JOIN NULL), UI **—**, sort **NULLS LAST**.
7. **P655-7** — Default `/pool` paint does not add LCD pair queries. Production still omits gems (**P562-3**).
8. **P655-8** — Manage expand `colSpan` matches the new column count (7). Invalid `sort` (including `tvl`, injection) → **400**. `limit=-1` clamps to 1.

## Verify

```bash
make verify-issue-655
```

Related: `make verify-issue-547` · `make verify-issue-569`.

## Related

- [`AGENTS_FRONTEND_POOL_TABLE.md`](./AGENTS_FRONTEND_POOL_TABLE.md) — table chrome, catalog default, A8
- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) — global TVL formula (**P569**)
- [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) — hub marks for UST1/USTR
- [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) — production gem hide
- [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) — no nested chrome
- GitLab [#664](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/664) — same `pair_liquidity_usd` stamp on **single-pair** `GET /api/v1/pairs/{addr}` (list JOIN + `/pool` column stay this issue)
