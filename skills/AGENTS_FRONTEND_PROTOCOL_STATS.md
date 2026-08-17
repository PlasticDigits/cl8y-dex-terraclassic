# Agent playbook: `/protocol` global USD stats + unified oracle (GitLab #550)

Audience: third-party agents changing Protocol page layout, overview JSON, or external oracle tickers.

**Issue:** [GitLab **#550**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550)  
**Oracle skill:** [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md) (**X1–X6**, now `ustc` \| `lunc` \| `vfdusd`)  
**Overview runbook:** [`docs/runbooks/overview-global-stats-brin.md`](../docs/runbooks/overview-global-stats-brin.md)  
**Frontend:** [`docs/frontend.md`](../docs/frontend.md) § Protocol

## Problem class

`/protocol` was USTC-only with two oracle panels and no DEX-wide USD dashboard. Charts already showed mixed-unit 24h `total_volume_24h`. Protocol is the USD census page.

## Invariants (P550)

| ID | Rule |
|----|------|
| **P550-1** | Page order: title → **Global stats** (`protocol-global-stats`) → **one** oracle card (`protocol-oracle`) → audit contracts → hooks. |
| **P550-2** | Oracle chips/tabs only `ustc` \| `lunc` \| `vfdusd`. `?ticker=` allowlisted; unknown / `javascript:` / `../` → `ustc`. |
| **P550-3** | Snapshot + sources + history live in **one** card. Query keys include ticker. |
| **P550-4** | Stats headline **USD** (`total_volume_*_usd`). Do **not** present mixed-unit `total_volume_24h` as volume. |
| **P550-5** | 7d/30d/active-pair/unique-trader figures come from `global_stats_24h` rollup + 60s cache. Cache-miss must not `SUM`/`COUNT(DISTINCT)` 30d `swap_events`. |
| **P550-6** | `token_count` is unique pair-leg assets (`count_pair_leg_assets`, [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548) **C6**), not `get_all_assets().len()`. New-token census is `tokens_added_30d` on `assets.created_at`. |
| **P550-7** | “New in 30d” is indexer `created_at` (first-seen). Reindex/rebuild makes everything look new — copy must not say “launched on chain”. |
| **P550-8** | Active pairs = distinct `pair_id` with ≥1 swap in last **24h** (materialized). Dust swaps count. Not unique traders. Not TVL. |
| **P550-9** | vFDUSD polls CEX **FDUSD** (`first-digital-usd` / `FDUSDUSDT`). Not `$1`. Not USTC/LUNC ids. No `fdusd` path alias. |
| **P550-10** | **X4** is P522-Q catalog (#548): UST1/USTC/LUNC/USTR legs can price `volume_usd`. Do **not** convert DEX volume with vFDUSD/FDUSD. Overview `ustc_price_usd` stays the USTC ticker. |
| **P550-11** | Feeds labeled **reference**. Not TWAP (Charts), not UST1 window (`/ust1`). |
| **P550-12** | Factory/router `AddressRow` stay on `/protocol` only (#378). |

## Do / don’t

- **Do** call `getOraclePrice(ticker)` / `getOracleHistory({ ticker })` with an allowlisted ticker.
- **Do** keep Charts overview strip additive-compatible (new fields optional on `IndexerOverview`).
- **Don’t** restore a second “Recent USTC/USD history” panel.
- **Don’t** live-scan `swap_events` on `GET /overview`.
- **Don’t** hardcode vFDUSD `$1`.
- **Don’t** clone Protocol audit rows onto Swap confirmation.

## Regression

```bash
make verify-issue-550
make verify-issue-515   # catalog still catalogs; X1–X6
```

## Related

- [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md)
- [`AGENTS_UST1_WINDOW_UI.md`](./AGENTS_UST1_WINDOW_UI.md) — different oracle
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — #489; Protocol stays short “reference” labels, not TWAP vs CEX essays
