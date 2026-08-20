# Agent playbook: `/protocol` global USD stats + unified oracle (GitLab #550 / #569)

Audience: third-party agents changing Protocol page layout, overview JSON, or external oracle tickers.

**Issue:** [GitLab **#550**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550) · [**#569**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/569) (pool TVL + 24h/30d Δ%)  
**Oracle skill:** [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md) (**X1–X6**, now `ustc` \| `lunc` \| `vfdusd`)  
**Overview runbook:** [`docs/runbooks/overview-global-stats-brin.md`](../docs/runbooks/overview-global-stats-brin.md)  
**Frontend:** [`docs/frontend.md`](../docs/frontend.md) § Protocol

## Problem class

`/protocol` was USTC-only with two oracle panels and no DEX-wide USD dashboard. Charts already showed mixed-unit 24h `total_volume_24h`. Protocol is the USD census page. Volume answers “how much traded”; **#569** adds **how much is locked in pools** and whether that stock is growing.

## Invariants (P550)

| ID | Rule |
|----|------|
| **P550-1** | Page order: title → **Global stats** (`protocol-global-stats`) → **DEX hub prices** (`protocol-dex-hub-prices`, [#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556) / [#570](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/570) cUSTC+LUNC wrap identity) → **one** CEX oracle card (`protocol-oracle`) → audit contracts → hooks. |
| **P550-2** | Oracle chips/tabs only `ustc` \| `lunc` \| `vfdusd`. `?ticker=` allowlisted; unknown / `javascript:` / `../` → `ustc`. |
| **P550-3** | Snapshot + sources + history live in **one** card. Query keys include ticker. |
| **P550-4** | Stats headline **USD** (`total_volume_*_usd`). Do **not** present mixed-unit `total_volume_24h` as volume. |
| **P550-5** | 7d/30d/active-pair/unique-trader **and** pool TVL / Δ% figures come from `global_stats_24h` rollup + 60s cache. Cache-miss must not `SUM`/`COUNT(DISTINCT)` 30d `swap_events`, join `pair_reserves`, or walk `global_liquidity_snapshots`. |
| **P550-6** | `token_count` is unique pair-leg assets (`count_pair_leg_assets`, [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548) **C6**), not `get_all_assets().len()`. New-token census is `tokens_added_30d` on `assets.created_at`. |
| **P550-7** | “New in 30d” is indexer `created_at` (first-seen). Reindex/rebuild makes everything look new — copy must not say “launched on chain”. |
| **P550-8** | Active pairs = distinct `pair_id` with ≥1 swap in last **24h** (materialized). Dust swaps count. Not unique traders. Not TVL. |
| **P550-9** | Path `vfdusd` polls CEX **FDUSD** (`first-digital-usd` / `FDUSDUSDT`). Indexer logs/API `display_name` is **FDUSD/USD**, not `vFDUSD/USD` ([#580](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/580)). Protocol tab heading is **vFDUSD**; CEX StatBox is **FDUSD reference price**. Not `$1`. Not USTC/LUNC ids. No `fdusd` path alias. Venus redeem is **#571** (`1 vFDUSD Price`). |
| **P550-10** | **X4** is P522-Q catalog (#548 / #556): USTC/LUNC oracles plus hub USD for UST1/USTR. Do **not** convert DEX volume **or TVL** with vFDUSD/FDUSD. Overview `ustc_price_usd` stays the USTC ticker; hub fields are additive. |
| **P550-11** | Feeds labeled **reference**. Not TWAP (Charts), not UST1 window (`/ust1`). |
| **P550-12** | Factory/router `AddressRow` stay on `/protocol` only (#378). |

## Invariants (P569)

| ID | Rule |
|----|------|
| **P569-1** | **Total liquidity** is humanized AMM pool TVL (`total_liquidity_usd` from `pair_reserves`). Not volume, not CG `liquidity_in_usd`, not raw reserves, not book escrow / parked dust. Testids: `protocol-stat-liquidity`, `protocol-stat-liquidity-24h`, `protocol-stat-liquidity-30d`. |
| **P569-2** | 24h/30d Δ% come from indexer snapshots (`liquidity_change_*_pct`). Missing / `null` / non-finite → em-dash (`formatProtocolPct`). Never `0%` / `Infinity` / client `now/then` without a zero guard. |
| **P569-3** | GET `/overview` stays O(1) rollup. Compute TVL on the aggregator / hub refresh, not on the request path. Snapshot insert is periodic; prune ≥ 35 days. |
| **P569-4** | Same USD catalog as volume: USTC/cUSTC/`uusd` → USTC oracle; LUNC/cLUNC/`uluna` → LUNC; UST1/USTR → `hub_prices`. Never `$1` UST1 or `2.5×` USTR. Oracle/hub down → omit that handle. |
| **P569-5** | Both legs priced → `h0×usd0 + h1×usd1`. Exactly one catalogued → `2×` that leg (CPAMM). Neither → omit. Omitted ≠ `$0`. Identity is contract/denom (A1); spoof natives skipped. |
| **P569-6** | Humanize decimals (`humanize_raw_amount` + `fits_numeric_38_18`). Overflow / non-positive → skip the pair. Double-count across pools is correct. |
| **P569-7** | Boxes live **inside** `protocol-global-stats`. Keep volume boxes. Do not headline `unique_traders_24h`. Charts overview strip stays additive-compatible. |
| **P569-8** | Cold start / `--fresh` / indexer younger than 24h/30d → Δ% empty until windows fill. Copy must not claim on-chain 30d genesis TVL. Flash LP inside one snapshot interval may move current TVL; Δ% uses snapshots. |

## Do / don’t

- **Do** call `getOraclePrice(ticker)` / `getOracleHistory({ ticker })` with an allowlisted ticker.
- **Do** keep Charts overview strip additive-compatible (new fields optional on `IndexerOverview`).
- **Do** reuse hub / P522-Q helpers for TVL; insert snapshots on successful refresh.
- **Don’t** restore a second “Recent USTC/USD history” panel.
- **Don’t** live-scan `swap_events` or `pair_reserves` on `GET /overview`.
- **Don’t** invent TVL from CG `liquidity_in_usd` or `total_volume_*`.
- **Don’t** hardcode vFDUSD `$1` or peg UST1 at `$1`.
- **Don’t** clone Protocol audit rows onto Swap confirmation.
- **Don't** title the CEX snapshot **vFDUSD / USD** (tab heading is **vFDUSD**; CEX box is **FDUSD reference price** — [#571](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/571)).
- **Don't** backfill 30d Δ% from zeros or `liquidity_events` after `--fresh`.

Trailing 24h / 7d / 30d **volume labels** are a trailing window, not calendar buckets ([#576](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/576), [`AGENTS_FRONTEND_TRAILING_WINDOW.md`](./AGENTS_FRONTEND_TRAILING_WINDOW.md)). Do not add a lecture to the Global stats lead.

## Regression

```bash
make verify-issue-569
make verify-issue-550
make verify-issue-556   # hub card still after stats
make verify-issue-515   # catalog still catalogs; X1–X6
make verify-issue-571   # FDUSD reference + Venus 1 vFDUSD Price
```

## Related

- [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md)
- [`AGENTS_INDEXER_VENUS_VFDUSD.md`](./AGENTS_INDEXER_VENUS_VFDUSD.md) — Venus `exchangeRateStored` on the vFDUSD tab ([#571](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/571))
- [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) — DEX hub card + `GET /api/v1/hub-prices` (#556)
- [`AGENTS_FRONTEND_PROTOCOL_HUB.md`](./AGENTS_FRONTEND_PROTOCOL_HUB.md) — cUSTC/cLUNC wrap `AddressRow` + LUNC column (#570)
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — P522-Q catalog used for TVL legs
- [`AGENTS_UST1_WINDOW_UI.md`](./AGENTS_UST1_WINDOW_UI.md) — different oracle
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — #489; Protocol stays short “reference” labels, not TWAP vs CEX essays
- [`AGENTS_FRONTEND_TRAILING_WINDOW.md`](./AGENTS_FRONTEND_TRAILING_WINDOW.md) — 24h/7d/30d volume is trailing, not calendar (#576)
