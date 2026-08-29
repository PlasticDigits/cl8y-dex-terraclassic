# Agent playbook: `/pool` sortable table (GitLab #547 / #655 / #692)

Audience: third-party agents changing the pool list, default rank, Charts deep links, `/pool` page chrome, the **v2 LP USD** column, or the **Vol** (24h USD) column.

**Issue:** [GitLab **#547**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547) · [**#655**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/655) (v2 LP USD) · [**#692**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/692) (Vol USD)  
**Invariants:** [`docs/frontend.md` § Liquidity pools list](../docs/frontend.md#liquidity-pools-list-indexer-vs-factory) (**P547-1–P547-10**, **P655-1–P655-8**, **PVol-1–PVol-8**)  
**Related:** [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/489) no lectures, [#531](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/531) how-to dismiss, [#534](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534) catalog rank, [#541](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/541) identity, [#537](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537) I14 fee chrome, [#569](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/569) protocol TVL math

## Problem class

`/pool` was a stack of `PoolCard`s with Sort/Order dropdowns, A–Z default (gems interleaved with UST1), no Charts URL, and indexer/factory lectures in the header. Router-known filtered only the current page.

## Do / don’t

- **Do** list pairs in a `<table>` with caret-by-label sortable headers (`PoolPairsTable`).
- **Do** default empty browse to catalog rank (`sortIndexerPairsByCatalog`) after fetching a large `volume_24h` window (`POOL_CATALOG_FETCH_LIMIT`). Production also **omits gems** from catalog and column/search pages ([#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562) **P562-3**).
- **Do** use indexer `sort`/`order` for user column clicks — **no** catalog overlay on Vol / **v2 LP USD** / Fee / Pair / Created.
- **Do** keep typed search on indexer `relevance` / `q` (**P534-6**).
- **Do** `Link` Charts to `/charts/:pairAddr` only via `chartsPairHref` (valid Terra bech32).
- **Do** mount `PoolAdvancedManage` only when **Manage** is expanded (A8: no N+1 LCD on default paint).
- **Do** render **v2 LP USD** from list JSON `liquidity_usd` via `formatProtocolUsd` (missing/`null`/non-finite → **—**).
- **Do** render **Vol** from list JSON `volume_usd_24h` via `formatPairListVolumeUsd` (missing/`null`/hostile → **—**).
- **Don’t** restore stacked cards as the only list, Sort/Order dropdowns, Router-known, or header “List source” / pair-count essays.
- **Don’t** put LCD `getPool` / `getPairFeeConfig` on every row for first paint.
- **Don’t** advertise CL8Y discounts from indexer `fee_bps` (I14 stays on Manage expand).
- **Don’t** invent pool USD from `volume_quote_24h`, CG `liquidity_in_usd`, hub-prices in the browser, or wallet LP balances ([#655](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/655) / [#692](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/692)).

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/pages/PoolPage.tsx` | Search + table; no page title; zap under Manage |
| `frontend-dapp/src/components/pool/PoolPairsTable.tsx` | Sortable table + Charts `Link` + Manage expand |
| `frontend-dapp/src/components/pool/PoolAdvancedManage.tsx` | Four peer actions; LCD on expand |
| `frontend-dapp/src/utils/poolListQuery.ts` | Catalog window, column sorts, pagination |
| `frontend-dapp/src/utils/chartsPairRoute.ts` | `/charts/:pairAddr` validation + href |
| `frontend-dapp/src/pages/ChartsPage.tsx` | Reads `:pairAddr`; invalid/unknown notices |
| `frontend-dapp/src/utils/poolLpHowto.ts` | Section dismiss `cl8y-dex-pool-lp-howto-section-dismissed` |

## Invariants (P547-1–P547-10)

1. **P547-1** — Primary list is a table (`role`/`<table>`), not stacked `PoolCard`.
2. **P547-2** — Sortable headers: label + caret; `aria-sort` on the active column; no Order dropdown.
3. **P547-3** — Default (no search, no column click) is catalog: UST1 hub first, gems last (**P534-1–P534-4**). Implementation: fetch `limit=500` `sort=volume_24h&order=desc`, client `sortIndexerPairsByCatalog` (USD 24h within hub when present), client-paginate 20.
4. **P547-4** — Column sort uses indexer keys only (`symbol`, `volume_usd_24h`, `liquidity_usd`, `fee`, `created`). Vol format is `formatPairListVolumeUsd`. Header stays **Vol** with trailing-24h USD `title` ([#576](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/576) / [#692](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/692)). Created cells show relative age from list `created_at` ([#662](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/662) **P662-3**); `—` only when missing/invalid. **v2 LP USD** is **P655**. Catalog fetch still uses `sort=volume_24h` (**P547-3**).
5. **P547-5** — Charts href is same-origin `/charts/<pairAddr>`. Invalid bech32 / `javascript:` / HTML → no `Link`.
6. **P547-6** — No Router-known checkbox or `pool-filter-router`.
7. **P547-7** — How-to section (hint + details) is dismissible; `#lp-howto` restores (**H531-7**).
8. **P547-8** — No “Liquidity Pools” `h2`, list-source essay, indexer/factory counts, or header eligibility note. Search + outage/registry banners stay. Zap is **not** page chrome (#660).
9. **P547-9** — Default table paint does not `getPool`/`getPairFeeConfig` per row. I14 fee badge + unregistered CTA + action forms on Manage expand.
10. **P547-10** — `#489`: no always-on architecture lectures. Factory/Indexer is a compact mark, not a filter.

## Invariants (P655-1–P655-8)

1. **P655-1** — **v2 LP USD** sits immediately after **Vol**. Compact USD or **—**. Not buried after Source.
2. **P655-2** — Cell is `formatProtocolUsd(liquidity_usd)`. Never `Infinity` / `NaN` / raw 18-dec / quote volume.
3. **P655-3** — Values come from indexer `liquidity_usd` only. Old indexers omit the field → all **—**.
4. **P655-4** — First header click `sort=liquidity_usd&order=desc`; toggle asc/desc; `aria-sort` on that `<th>` only. Catalog default unchanged.
5. **P655-5** — Priced UST1/cUSTC (or UST1/USTR) matches `protocol_pair_tvl` — not `$1`/leg, not `2.5×` USTR.
6. **P655-6** — Unpriced → **—** and sorts after priced rows (indexer **NULLS LAST**).
7. **P655-7** — Default paint: zero new `getPool` / `getPairFeeConfig`. Manage `colSpan` matches the new column count.
8. **P655-8** — Production hide-gems still applies. No nested `card-glass`. Header is not a TVL lecture.

## Verify

```bash
make verify-issue-547
make verify-issue-655
make verify-issue-692
make verify-issue-662
```

## Related

- [`AGENTS_FRONTEND_POOL_CREATED.md`](./AGENTS_FRONTEND_POOL_CREATED.md) — Created relative age from list `created_at` (**P662-1–P662-8**, [#662](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/662))
- [`AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](./AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) — catalog overlay; `/pool` default is in scope
- [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) — production omits gems from `/pool` too ([#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562))
- [`AGENTS_FRONTEND_POOL_LP_HOWTO.md`](./AGENTS_FRONTEND_POOL_LP_HOWTO.md) — whole-section dismiss
- [`AGENTS_FRONTEND_POOL_MANAGE_IA.md`](./AGENTS_FRONTEND_POOL_MANAGE_IA.md) — four peer Manage actions (#660)
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — no header lectures
- [`AGENTS_FRONTEND_TRAILING_WINDOW.md`](./AGENTS_FRONTEND_TRAILING_WINDOW.md) — Vol header `title` is trailing 24h, not midnight reset (#576)
- [`AGENTS_FRONTEND_TOKEN_IDENTITY.md`](./AGENTS_FRONTEND_TOKEN_IDENTITY.md) — identity on the table row
- [`AGENTS_FRONTEND_TRADE_IDENTITY_LP.md`](./AGENTS_FRONTEND_TRADE_IDENTITY_LP.md) — Trade / Charts **v2 LP** chip (#664); do **not** add that chip on `/pool` rows (#655)
- [`AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md`](./AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md) — I14 on expand, not header
- [`AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md`](./AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md) — list JSON `liquidity_usd` rollup (#655)
- [`AGENTS_INDEXER_PAIR_VOLUME_USD.md`](./AGENTS_INDEXER_PAIR_VOLUME_USD.md) — list JSON `volume_usd_24h` + `/pool` Vol (#692). Coolify leftover: [#701](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/701). Coolify leftover: [`AGENTS_POST_MERGE_OPS_701.md`](./AGENTS_POST_MERGE_OPS_701.md) ([#701](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/701))
- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) — global pool TVL sum (#569); `/pool` shows the per-pair stamp
- [`AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md`](./AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md) — Advanced provide name/symbol + wrap default (#661)
