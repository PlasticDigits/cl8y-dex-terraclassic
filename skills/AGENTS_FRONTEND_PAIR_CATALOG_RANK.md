# Agent playbook: Pair catalog rank (GitLab #534)

Audience: third-party agents touching pair/token pickers, 24h volume badges, Trade auto-pick, or Charts pair `MenuSelect`.

**Issue:** [GitLab **#534**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534)  
**Invariants:** [`docs/frontend.md` § Pair catalog rank](../docs/frontend.md#pair-catalog-rank) (**P534-1–P534-8**)  
**Related:** [#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522) human prices vs raw volume, [#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508) UST1 pairs (**U6** gems stay gems)

## Problem class

Factory `pairs` (and indexer `sort=volume_24h` when volumes are 0 or raw 18-dec) lists **UST1/USTR**, **UST1/cUSTC**, then faucet gems, then **cLUNC/UST1** last. The vol badge used `formatNum(raw)` so USTR (18-dec) showed **VOL 19,297,048T**.

## Do / don’t

- **Do** rank empty pair browse via [`pairCatalogRank.ts`](../frontend-dapp/src/utils/pairCatalogRank.ts) (`sortPairInfosByCatalog` / `sortIndexerPairsByCatalog`).
- **Do** format `volume_quote_24h` with [`formatQuoteVolume24h`](../frontend-dapp/src/utils/formatAmount.ts) and **quote** (`asset_1`) decimals.
- **Do** keep typed pair search on indexer `relevance` / local haystack (**P534-6**).
- **Do** auto-pick bare `/trade` with `firstCatalogPairAddress`.
- **Don’t** treat UST1 / CL8Y / wrap tokens as gems (**U6** / **P534-8**).
- **Don’t** pass raw 18-dec volume into `formatNum` (compact `T` is not a volume formatter here).
- **Don’t** change indexer JSON: `volume_quote_24h` stays a **raw** integer. Overlay rank in the dApp.
- **Don’t** collapse the **Test pairs** divider on LocalTerra (optional in #534; we show it, not collapsed). Production hides gems so the divider is absent ([#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562)).
- **Don’t** secretly re-apply catalog overlay on `/pool` **after** the user sorts by volume/fee/created/name ([#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547)). Default `/pool` **is** catalog-ranked.

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/pairCatalogRank.ts` | Gem set, hub ranks, human volume compare, sorts |
| `frontend-dapp/src/utils/formatAmount.ts` | `formatQuoteVolume24h` |
| `frontend-dapp/src/components/trade/PairSearchSelect.tsx` | Empty browse + vol badge + Test pairs divider |
| `frontend-dapp/src/pages/TradePage.tsx` | Catalog auto-pick |
| `frontend-dapp/src/pages/ChartsPage.tsx` | Catalog rank on pair `MenuSelect` |
| `frontend-dapp/src/utils/tokenSearchQuery.ts` | Swap token empty browse (**P534-7**) |
| `frontend-dapp/src/pages/PoolPage.tsx` | Default catalog table ([#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547)); human 24h vol via `formatQuoteVolume24h` |

## Rank (empty browse)

1. Economic pairs (both legs non-gem).
2. Within economic: hub UST1 → cLUNC → cUSTC → USTR → CL8Y → vFDUSD → other.
3. Within a hub: human 24h quote volume desc, then the other symbol.
4. Test pairs last, under **Test pairs**.

## Regression

```bash
make verify-issue-534
```

Vitest: `pairCatalogRank.test.ts`, `formatAmount.test.ts` (`formatQuoteVolume24h`), `pairSearchQuery.test.ts`, `tokenSearchQuery.test.ts`, `PairSearchSelect.issue534.test.tsx`, `PairSearchSelect.issue301.test.tsx`.

## Related

- [`AGENTS_FRONTEND_POOL_TABLE.md`](./AGENTS_FRONTEND_POOL_TABLE.md) — `/pool` default catalog + column sorts ([GitLab **#547**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547)); **v2 LP USD** is [#655](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/655); Created age [`AGENTS_FRONTEND_POOL_CREATED.md`](./AGENTS_FRONTEND_POOL_CREATED.md) ([#662](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/662))
- [`AGENTS_FRONTEND_TOKEN_SEARCH.md`](./AGENTS_FRONTEND_TOKEN_SEARCH.md) — Swap token combobox
- [`AGENTS_FRONTEND_CREATE_PAIR_PICKER.md`](./AGENTS_FRONTEND_CREATE_PAIR_PICKER.md) — Create Pair listed CW20s (not the factory graph) ([GitLab **#542**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/542))
- [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) — production hide of gems ([GitLab **#562**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562)); LocalTerra still ranks per **P534**
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — prices are human; **pair list volume is still raw**
- [`AGENTS_FRONTEND_CHARTS_PAIR_STATS.md`](./AGENTS_FRONTEND_CHARTS_PAIR_STATS.md) — Charts pair-detail 24h Vol (USD) + token vols + TWAP ([#565](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/565) / [#564](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/564))
- [`AGENTS_UST1_SECONDARY_AMM.md`](./AGENTS_UST1_SECONDARY_AMM.md) — **U6** do not fold UST1 into gems
- [`AGENTS_FRONTEND_TRADE_PAIR_SWITCH.md`](./AGENTS_FRONTEND_TRADE_PAIR_SWITCH.md) — pair switch latency unchanged
