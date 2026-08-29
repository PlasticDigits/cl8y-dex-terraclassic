# Agent playbook: `/pool` Vol USD + pair-list 24h volume stamp (GitLab #692)

Audience: third-party agents changing `/pool` Vol, pair-search volume badges, `GET /api/v1/pairs` JSON, or `pair_volume_24h`.

**Issue:** [GitLab **#692**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/692) (leftover of [#544](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/544) on **pair-list** surfaces)  
**Invariants:** [`docs/frontend.md` § Liquidity pools list](../docs/frontend.md#liquidity-pools-list-indexer-vs-factory) (**PVol-1–PVol-8**), [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (pair list 24h volume)  
**Parent table:** [`AGENTS_FRONTEND_POOL_TABLE.md`](./AGENTS_FRONTEND_POOL_TABLE.md) (**P547**)  
**Catalog rank:** [`AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](./AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) (**P534**)  
**Stock column (not this):** [`AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md`](./AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md) (**P655** — v2 LP USD)

## Problem class

`/pool` **Vol** printed human **quote-token** 24h volume. Retailers cannot compare UST1/cUSTC vs UST1/USTR. Pair-detail stats already had `volume_usd`; the **list** JSON did not. **v2 LP USD** is pool *stock*; this issue is 24h *flow* in USD.

## Do / don’t

- **Do** stamp `pair_volume_24h.volume_usd = SUM(swap_events.volume_usd)` inside `refresh_pair_volumes` (~5 min). Reuse ingest (`volume_usd_for_swap` / hub, #548 / #556). One notional per consolidated swap (**L10**).
- **Do** JOIN that column on `GET /api/v1/pairs`. Sort key `volume_usd_24h` (default desc, **NULLS LAST**).
- **Do** keep `volume_quote_24h` **raw** and `sort=volume_24h` as quote-raw.
- **Do** render **Vol** with `formatPairListVolumeUsd` (compact `$` or **—**).
- **Don’t** live-`SUM(swap_events)` on GET (including `sort=volume_usd_24h`).
- **Don’t** invent USD from `volume_quote_24h`, hub-prices, CG, `$1` UST1, or `2.5×` USTR.
- **Don’t** N+1 `GET …/stats` from `/pool` (**P547-9**).
- **Don’t** `COALESCE` unpriced to `0` for display or sort (unpriced ranks last, cell **—**).
- **Don’t** accept `sort=volume_usd` (must be `volume_usd_24h`) — invalid → **400**.
- **Don’t** change CG/CMC volumes, candles, #522 price, or #655 stock.

## Canonical code

| File | Role |
|------|------|
| `indexer/migrations/20260829120000_pair_volume_24h_usd.sql` | Nullable `volume_usd` + `NULLS LAST` index |
| `indexer/src/db/queries/volume.rs` | `refresh_pair_volumes` stamps USD + zeros idle |
| `indexer/src/db/queries/pairs.rs` | List SELECT + `PairListSort::VolumeUsd24h` |
| `indexer/src/api/pairs.rs` | Additive `volume_usd_24h` + sort allowlist |
| `frontend-dapp/src/components/pool/PoolPairsTable.tsx` | Vol cell + header sort key `volume_usd_24h` |
| `frontend-dapp/src/utils/poolListQuery.ts` | `POOL_COLUMN_SORTS` Vol → `volume_usd_24h` |
| `frontend-dapp/src/utils/chartsOverviewStats.ts` | `formatPairListVolumeUsd` |
| `frontend-dapp/src/components/trade/PairSearchSelect.tsx` | Badge `vol $…` |
| `frontend-dapp/src/utils/pairCatalogRank.ts` | USD-first within hub |

## Invariants (PVol-1–PVol-8)

1. **PVol-1** — `/pool` **Vol** is compact USD or **—**. Never raw 18-dec or quote-token without `$`.
2. **PVol-2** — List JSON `volume_usd_24h` only. No per-row `/stats` / LCD / hub-prices / quote × anything.
3. **PVol-3** — Additive JSON. `volume_quote_24h` raw. Unpriced omit/`null`. `sort=volume_usd_24h` **NULLS LAST**. `sort=volume_24h` still quote.
4. **PVol-4** — First Vol click `sort=volume_usd_24h&order=desc`. Catalog default unchanged (**P547-3**).
5. **PVol-5** — List USD ≈ pair `…/stats` `volume_usd` within ~5 min. Header title is trailing 24h USD.
6. **PVol-6** — Unpriced / overflow → **—** / sort last. Idle → **—**. Production gems still hidden.
7. **PVol-7** — Default paint: zero new LCD / `/stats`. Manage `colSpan` 7. Same USD on Manage + pair-search.
8. **PVol-8** — Invalid `sort` → **400**. `limit=-1` clamps to 1. Hostile strings → **—**.

## Verify

```bash
make verify-issue-692
```

Playwright pool-table uses dedicated Vite `:3173` (indexer CORS). Do not leak a different `PLAYWRIGHT_WEB_PORT`.

Coolify leftover migrate + live list↔stats is [#701](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/701) — [`AGENTS_POST_MERGE_OPS_701.md`](./AGENTS_POST_MERGE_OPS_701.md). Do **not** reopen this issue for ops/QA.

Coolify leftover migrate + live list↔stats is [#701](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/701) — [`AGENTS_POST_MERGE_OPS_701.md`](./AGENTS_POST_MERGE_OPS_701.md). Do **not** reopen #692 for ops/QA.

Related: `make verify-issue-547` · `make verify-issue-655` · `make verify-issue-534` · `make verify-issue-576`.

## Related

- [`AGENTS_FRONTEND_POOL_TABLE.md`](./AGENTS_FRONTEND_POOL_TABLE.md) — table chrome, catalog default, A8
- [`AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](./AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) — within-hub USD then human-quote fallback
- [`AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md`](./AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md) — stock USD (not Vol)
- [`AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md`](./AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md) — idle zero (**D3**)
- [`AGENTS_FRONTEND_TRAILING_WINDOW.md`](./AGENTS_FRONTEND_TRAILING_WINDOW.md) — Vol `title` is trailing 24h
- [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) — hub marks used at **ingest**, not in the browser
