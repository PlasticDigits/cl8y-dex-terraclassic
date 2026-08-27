# Agent playbook: `/protocol` global USD stats + unified oracle (GitLab #550 / #569 / #586 / #652 / #667 / #668 / #677)

Audience: third-party agents changing Protocol page layout, overview JSON, or external oracle tickers.

**Issue:** [GitLab **#550**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550) · [**#569**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/569) (pool TVL + 24h snapshot Δ%) · [**#586**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586) (treasury fees) · [**#652**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652) (inline Δ% + volume prior-window % + UTC-day series) · [**#667**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/667) (Δ% grouped with headline; integer census) · [**#668**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/668) (USD axis + Hourly/Daily/Monthly grain chart) · [**#677**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/677) (liquidity 24h-only + denser UTC x-axis) · [**#613**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/613) (wrap/unwrap ingest) · [**#614**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614) (UST1 window mint/redeem fees)  
**Oracle skill:** [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md) (**X1–X6**, now `ustc` \| `lunc` \| `vfdusd`)  
**Overview runbook:** [`docs/runbooks/overview-global-stats-brin.md`](../docs/runbooks/overview-global-stats-brin.md)  
**Frontend:** [`docs/frontend.md`](../docs/frontend.md) § Protocol

## Problem class

`/protocol` was USTC-only with two oracle panels and no DEX-wide USD dashboard. Charts already showed mixed-unit 24h `total_volume_24h`. Protocol is the USD census page. Volume answers “how much traded”; **#569** adds **how much is locked in pools** and whether that stock is growing.

## Invariants (P550)

| ID | Rule |
|----|------|
| **P550-1** | Page order: title → **Global stats** (`protocol-global-stats`) → **Protocol fees** (`protocol-fee-stats`, [#586](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586)) → **DEX hub prices** (`protocol-dex-hub-prices`, [#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556) / [#570](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/570) cUSTC+LUNC wrap identity) → **one** CEX oracle card (`protocol-oracle`) → audit contracts → hooks. |
| **P550-2** | Oracle chips/tabs only `ustc` \| `lunc` \| `vfdusd`. `?ticker=` allowlisted; unknown / `javascript:` / `../` → `ustc`. |
| **P550-3** | Snapshot + sources + history live in **one** card. Query keys include ticker. |
| **P550-4** | Stats headline **USD** (`total_volume_*_usd`). Do **not** present mixed-unit `total_volume_24h` as volume. |
| **P550-5** | 7d/30d/active-pair/unique-trader **and** pool TVL / Δ% **and** fee totals come from `global_stats_24h` rollup + 60s cache. Cache-miss must not `SUM`/`COUNT(DISTINCT)` 30d `swap_events`, join `pair_reserves`, walk `global_liquidity_snapshots`, or scan `protocol_fee_events`. |
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
| **P569-1** | **Total liquidity** is humanized AMM pool TVL (`total_liquidity_usd` from `pair_reserves`). Not volume, not CG `liquidity_in_usd`, not raw reserves, not book escrow / parked dust. One cell: USD + **one** inline 24h snapshot Δ% (`protocol-stat-liquidity-24h`). Do **not** render `protocol-stat-liquidity-30d` on this tile ([#677](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/677)). `liquidity_change_30d_pct` stays on `GET /overview` for API consumers. |
| **P569-2** | 24h/30d Δ% come from indexer snapshots (`liquidity_change_*_pct`). Missing / `null` / non-finite → em-dash (`formatProtocolPct`). Never `0%` / `Infinity` / client `now/then` without a zero guard. |
| **P569-3** | GET `/overview` stays O(1) rollup. Compute TVL on the aggregator / hub refresh, not on the request path. Snapshot insert is periodic; prune ≥ 35 days. |
| **P569-4** | Same USD catalog as volume: USTC/cUSTC/`uusd` → USTC oracle; LUNC/cLUNC/`uluna` → LUNC; UST1/USTR → `hub_prices`. Never `$1` UST1 or `2.5×` USTR. Oracle/hub down → omit that handle. |
| **P569-5** | Both legs priced → `h0×usd0 + h1×usd1`. Exactly one catalogued → `2×` that leg (CPAMM). Neither → omit. Omitted ≠ `$0`. Identity is contract/denom (A1); spoof natives skipped. |
| **P569-6** | Humanize decimals (`humanize_raw_amount` + `fits_numeric_38_18`). Overflow / non-positive → skip the pair. Double-count across pools is correct. |
| **P569-7** | Liquidity + volume + census live **inside** `protocol-global-stats`. Volume tiles show USD + prior-window Δ% in the same cell. Δ% is visually grouped with its headline (`justify-start` / wrap via `.stat-value-row`), not spaced `justify-between` to the next column ([#667](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/667) **P667-1**). Optional UTC grain bar chart (`protocol-volume-daily-chart`, [#668](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/668)) sits under the volume row. Do not headline `unique_traders_24h`. Charts overview strip stays additive-compatible. Metric cells use `StatBox variant="flat"` — no nested `card-glass` ([#652](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652)). |
| **P569-7** | Liquidity + volume + census live **inside** `protocol-global-stats`. Volume tiles show USD + prior-window Δ% in the same cell. Optional UTC-day bar chart (`protocol-volume-daily-chart`) sits under the volume row. Do not headline `unique_traders_24h`. `/charts` does not render this census ([#666](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666)). New overview JSON fields stay optional on `IndexerOverview`. Metric cells use `StatBox variant="flat"` — no nested `card-glass` ([#652](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652)). |
| **P569-8** | Cold start / `--fresh` / indexer younger than 24h/30d → Δ% empty until windows fill. Copy must not claim on-chain 30d genesis TVL. Flash LP inside one snapshot interval may move current TVL; Δ% uses snapshots. |

## Do / don’t

- **Do** call `getOraclePrice(ticker)` / `getOracleHistory({ ticker })` with an allowlisted ticker.
- **Do** keep `GET /overview` additive-compatible (new fields optional on `IndexerOverview`). `/charts` must not grow a census strip ([#666](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/666)).
- **Do** reuse hub / P522-Q helpers for TVL; insert snapshots on successful refresh.
- **Don’t** restore a second “Recent USTC/USD history” panel.
- **Don’t** live-scan `swap_events` or `pair_reserves` on `GET /overview`.
- **Don’t** invent TVL from CG `liquidity_in_usd` or `total_volume_*`.
- **Don’t** hardcode vFDUSD `$1` or peg UST1 at `$1`.
- **Don’t** clone Protocol audit rows onto Swap confirmation.
- **Don't** title the CEX snapshot **vFDUSD / USD** (tab heading is **vFDUSD**; CEX box is **FDUSD reference price** — [#571](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/571)).
- **Don't** backfill 30d Δ% from zeros or `liquidity_events` after `--fresh`.
- **Don't** treat volume Δ% as liquidity snapshot % or title it “liquidity.”
- **Don't** call `GET /defillama/daily` from `/protocol` or add Llama `from`/`to`.
- **Don't** nest `card-glass` inside `protocol-global-stats` / `protocol-fee-stats` metric grids.
- **Don't** scan `swap_events` on `GET /protocol/volume/daily` or accept free-form `from`/`to` / over-max `limit`.
- **Don't** mount `PriceChart` on the Protocol volume census chart, or restore `7d`/`30d` as the grain selector.
- **Don't** put a 30d Δ% chip back on Total liquidity, or restore `sparseTimeLabelIndexes` / `maxLabels = 5` ([#677](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/677)).

## Invariants (PFee — GitLab #586)

| ID | Rule |
|----|------|
| **PFee-1** | Fee panel `protocol-fee-stats` sits **after** Global stats and **before** DEX hub. Do not merge factory/router into fees. Do not headline `traders.total_fees_paid` (lifetime mixed-unit, includes spread). |
| **PFee-2** | Headlines are trailing **24h / 7d / 30d** treasury fee USD with **inline** flow Δ% vs the prior equal window (same cell; `protocol-stat-fees-*-chg` is a child, not a sibling card). Δ% sits immediately after the USD (`justify-start` / wrap), not in the gutter under the next fee label ([#667](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/667) **P667-1**). Idle → `$0`; activity + all unpriced → `—`; missing prior / `then ≤ 0` → Δ% `—`. Never `Infinity`. Flat `StatBox` inside `protocol-fee-stats` ([#652](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652)). |
| **PFee-3** | Source table uses retail labels (wrap / unwrap / **UST1 mint** / **UST1 redeem** / AMM swap / book take / limit place), not wasm action strings (`deposit` / `withdraw` / `effective_swap`). Unconfigured wrap mapper **omits** wrap/unwrap. Unconfigured `UST1_WINDOW_ADDRESS` (or missing `ust1_window_configured`) **omits** mint/redeem — not fake idle `$0`. Hide idle `$0` sources. |
| **PFee-4** | Token table is human units + USD, cap 8 + `other`. Unpriced token shows human + USD `—`. XSS/`javascript:` symbols render as **text**. |
| **PFee-5** | Hybrid fee = pool `commission_amount` (`swap_amm`) + `limit_order_fills.commission_amount` (`book_take`) — **not both** fill commission and swap `book_commission_amount`. Placement `maker_fee_amount` is `limit_place`. |
| **PFee-6** | Wrap/unwrap only from pinned `WRAP_MAPPER_ADDRESS` (exact `terra1` bech32). Captured ustr-cmm attrs (**I613** / [#613](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/613)): wrap execute is `action=notify_deposit` (not `wrap`); amount key is **`fee`** (legacy `fee_amount` still accepted). Per-action segments + reserved `_contract_address`. Fail closed on missing `fee` / token identity. Burn tax / `tax_amount` / `hook_fee_amount` / spread / `wrap_deposit` / `instant_withdraw` are **not** protocol fees. |
| **PFee-7** | Same P522-Q / hub catalog as volume/TVL. Never vFDUSD. Never `$1` UST1 or `2.5×` USTR. Stamp `fee_usd` at ingest — do not rewrite from the live hub (#568). |
| **PFee-8** | GET `/overview` and GET `/protocol/fees` are O(1) rollup / 60s cache. Do **not** `SUM` `protocol_fee_events` / `swap_events` / fills on GET. `window=` allowlist `24h` \| `7d` \| `30d` → else **400**. `OVERVIEW_GLOBAL_STATS_LIVE=1` still must not 60d-SUM fees. |
| **PFee-9** | Windows decay when events age out (#577). `--fresh` / young indexer → Δ% empty until 2×W fills. Copy must not claim chain genesis fees. Dust swaps **do** count (same as volume). |
| **PFee-10** | Additive overview JSON. Missing fee fields (old indexer) → hide the fee panel, do not invent `$0`. Fee query is in `detectMarketDataOutage` / retry. `?ticker=` stay allowlisted; fee panel ignores ticker. |
| **PFee-11** | Breakdown cardinality is bounded (fixed source enum; top 8 tokens + `other`). No CSV in v1. |
| **PFee-12** | Verify: `make verify-issue-586`. Wrap ingest attrs: `make verify-issue-613`. Related: `make verify-issue-550` `569` `576` `577`. Post-merge stack: `make verify-issue-590` ([#590](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/590)). Window mint/redeem: `make verify-issue-614`. |
| **PFee-13** | UST1 window mint/redeem only from pinned `UST1_WINDOW_ADDRESS` (same terra1 pin rules as wrap; do **not** reuse `WRAP_MAPPER_ADDRESS`). Actions `deposit` → `ust1_mint`, `withdraw` → `ust1_redeem`. Require explicit `fee_amount` + token (`fee_asset` / `fee_denom` / `denom` / `ust1_token`). **Never** infer `ust1_out × fee_total_bps` / `vfdusd_to_treasury × fee_cmm_protocol_bps`. Columbus-5 **11566** crate attrs (`fee_*_bps` / `ust1_out` / `vfdusd_to_treasury`) are **not** a fee amount (fail closed). Same address **11618** ([ust1-window#33](https://gitlab.com/PlasticDigits/ust1-window/-/issues/33)) emits `fee_amount` + `fee_asset` (UST1). Flattened CW20 `send` + hook scopes by reserved `_contract_address` (#285). Price with hub UST1 (**PFee-7**); never vFDUSD/FDUSD / `$1` UST1. `window=` query param stays `24h`\|`7d`\|`30d` (not “ust1-window”). Coolify pin is the **indexer** `UST1_WINDOW_ADDRESS`, not only Vite `VITE_UST1_WINDOW_ADDRESS`. Playbook: [`AGENTS_INDEXER_UST1_WINDOW_FEES.md`](./AGENTS_INDEXER_UST1_WINDOW_FEES.md) (**I614-1–I614-8**). |

## Invariants (P652 — GitLab #652)

| ID | Rule |
|----|------|
| **P652-1** | Volume Δ% is **flow** (`flow_change_pct` vs prior equal window `[2W, W)`). Not TVL snapshots. `prior ≤ 0` / unpriced activity / overflow → JSON `null` / UI em-dash. Never Inf. Additive `volume_change_{24h,7d,30d}_pct` on `GET /overview`. |
| **P652-2** | GET `/overview` stays 60s cache + O(1) rollup. Volume priors are aggregator-only. `OVERVIEW_GLOBAL_STATS_LIVE=1` still must not 60d-SUM volume priors from `swap_events`. |
| **P652-3** | `GET /api/v1/protocol/volume/daily?days=` allowlist `7` \| `30` else **400** when `grain` is omitted. 60s cache keyed by allowlisted `days`. Reads `protocol_daily_volume` only. Do **not** N+1 Llama `GET /defillama/daily`. Do **not** add `from`/`to` to Llama. Daily prune ≥ 95d so `grain=daily&limit=90` can fill. |
| **P652-4** | Daily methodology = Protocol catalog (same as overview volume). Includes gems / wrap / window swaps. Idle day → `"0"`; activity + unpriced → `null`. Missing rollup row → idle `"0"`. Newest-last. Alias cap 30 points; grain daily cap **90** (**P668-4**). |
| **P652-5** | Chart is bars inside `protocol-global-stats` (`protocol-volume-daily-chart`). Hide on 404/501. Do not mount `PriceChart`. Selector is **Hourly / Daily / Monthly** (default Daily) — not `7d` / `30d`. Subtitle names the UTC bucket (hour / calendar day / calendar month). No unique-trader headline. Extended by **P668**. |
| **P652-6** | Census tiles stay value-only. Integers render as locale counts (`14`, not `14.00`); compact `K` only when `abs ≥ 1000` (`formatProtocolCount`, [#667](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/667) **P667-3**). Old indexer (missing keys / daily route) → tiles still render; Δ% em-dash; chart hidden. |
| **P652-7** | Verify: `make verify-issue-652`. Keep `verify-issue-550` / `569` / `586` / `576` / `577` / `631` green. Companion chrome pass: [#653](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653). Visual grouping: `make verify-issue-667`. Grain chart: `make verify-issue-668`. |

## Invariants (P667 — GitLab #667)

DOM parentage from [#652](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652) is not enough: `justify-between` pinned Δ% to the cell’s far right so operators read the chip as the **next** column’s figure. Grouping is layout, not a new stat.

| ID | Rule |
|----|------|
| **P667-1** | Value+Δ% row uses `.stat-value-row` (`justify-start` / wrap). Headline first; Δ% cluster immediately after. On wrap, chips go to the next line of the **same** cell, left-aligned under the `$`. Never `justify-between`. Never a sibling card. |
| **P667-2** | Chip bounding box stays inside its own `protocol-stat-*` tile (`chip.right ≤ tile.right` and, when the neighbor is on the same row, `chip.left < nextTile.left`). Phone 390 / tablet 820 wrap is OK. Liquidity keeps **one** 24h chip in `.stat-delta-cluster` ([#677](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/677)). Em-dash (`— 24h`) still groups with its headline. |
| **P667-3** | `formatProtocolCount` is integer census: finite ≥0 → locale string under 1000 (`14`, not `14.00`); compact `K` only when `abs ≥ 1000`; missing / non-finite / negative / XSS-like → em-dash. Census tiles stay value-only (**P652-6**). |
| **P667-4** | Verify: `make verify-issue-667`. Keep `verify-issue-652` / `550` / `569` / `586` / `653` / `677` green. No overview JSON / indexer / wasm change. No-Δ% `StatBox` call sites (Charts / Trader) stay label+value. |

Do **not** invent a Protocol-only `StatBox` variant. The same value-row class serves fees and any future Δ% tile. Do **not** add a lecture banner about “Δ% is next to the number.”


## Invariants (P668 — GitLab #668)

| ID | Rule |
|----|------|
| **P668-1** | Visible **USD value axis** (`formatProtocolUsd` ticks, including `$0`). Bars stay a UTC **time** series (vertical). Do **not** rotate into a horizontal-bar USD category chart. Never Inf / `NaN` / raw `uluna` on the axis. |
| **P668-2** | Pointer **and** keyboard focus on a bar show an in-document tooltip: UTC period + USD (unpriced → em-dash). Not SVG `<title>`-only. Tooltip is text (no `innerHTML` / `eval` / `javascript:` URLs). |
| **P668-3** | Selector **Hourly / Daily / Monthly** (default Daily). Trailing Last 24h / 7d / 30d tiles are unchanged (#576). Labels ≤ ~5 words (#489). |
| **P668-4** | `limit` follows plot width (`ResizeObserver`, ~12px slot), clamped per grain: hourly **12–168**, daily **7–90**, monthly **6–24**. Debounce resize; query key is `(grain, limit)`. Client rejects non-allowlisted grain/limit **before** fetch. |
| **P668-5** | GET `grain=hourly\|daily\|monthly` + integer `limit` (1..=max) else **400**. `from`/`to` → **400**. `days=7\|30` without `grain` stays the #652 alias. Cache 60s keyed only by allowlisted `(grain, limit)` or `days` (extra query junk cannot bust cache). |
| **P668-6** | GET reads rollup tables only (`protocol_hourly_volume` / `protocol_daily_volume` / `protocol_monthly_volume`). No `swap_events` SUM. No Llama. Idle `"0"`; activity + unpriced `null`. Missing row → idle `"0"`. Newest-last. |
| **P668-7** | Hourly bucket `[hour, hour+1)` UTC (`YYYY-MM-DDTHH`). Monthly = UTC calendar month (`YYYY-MM`), not trailing 30d. Aggregator refresh; `--fresh` / young indexer shows idle zeros / available months — **not** a GET-path backfill. Hourly prune ~10d; monthly retain ≥ 24 months. |
| **P668-8** | XSS strings in period / `volume_usd` render as **text**. Unpriced bars are outlined, not `$0`. No `PriceChart`. No nested `card-glass` around the plot (C653). Verify: `make verify-issue-668`. Keep `verify-issue-652` / `550` / `569` / `586` / `576` / `631` / `653` / `677` green. |
| **P668-9** | X-axis labels every bar (**step 1**) or every second bar (**step 2**) for Daily and Monthly. Hourly may use a wider step only when step 2 still collides in the fixed viewBox (`HH` vs slot). First and last period stay labeled. No global `maxLabels = 5`. Tooltip still tells the truth on unlabeled hourly bars ([#677](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/677)). |

Trailing 24h / 7d / 30d **volume labels** are a trailing window, not calendar buckets ([#576](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/576), [`AGENTS_FRONTEND_TRAILING_WINDOW.md`](./AGENTS_FRONTEND_TRAILING_WINDOW.md)). The volume chart is a **separate** UTC calendar-grain series (hour / day / month) — do not add a lecture to the Global stats lead.

## Regression

```bash
make verify-issue-613
make verify-issue-614
make verify-issue-677
make verify-issue-668
make verify-issue-652
make verify-issue-667
make verify-issue-586
make verify-issue-569
make verify-issue-550
make verify-issue-556   # hub card still after fees
make verify-issue-515   # catalog still catalogs; X4
make verify-issue-571   # FDUSD reference + Venus 1 vFDUSD Price
```

## Related

- [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md)
- [`AGENTS_INDEXER_VENUS_VFDUSD.md`](./AGENTS_INDEXER_VENUS_VFDUSD.md) — Venus `eth_call` of `exchangeRateCurrent` on the vFDUSD tab ([#571](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/571))
- [`AGENTS_INDEXER_WRAP_FEE_INGEST.md`](./AGENTS_INDEXER_WRAP_FEE_INGEST.md) — captured wrap-mapper attrs + flattened parse (**I613-1–I613-8**, [#613](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/613))
- [`AGENTS_INDEXER_UST1_WINDOW_FEES.md`](./AGENTS_INDEXER_UST1_WINDOW_FEES.md) — window `fee_amount` pin/parse (**I614-1–I614-8**, [#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614))
- [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) — DEX hub card + `GET /api/v1/hub-prices` (#556)
- [`AGENTS_FRONTEND_PROTOCOL_HUB.md`](./AGENTS_FRONTEND_PROTOCOL_HUB.md) — cUSTC/cLUNC wrap `AddressRow` + LUNC column (#570)
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — P522-Q catalog used for TVL legs
- [`AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md`](./AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md) — `/pool` per-pair TVL stamp (`liquidity_usd`, [#655](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/655))
- [`AGENTS_UST1_WINDOW_UI.md`](./AGENTS_UST1_WINDOW_UI.md) — `/ust1` execute; CEX/hub cards are **not** the window rate (**P550-11**). Window treasury fees: **PFee-13** / [#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614)
- [`AGENTS_POST_MERGE_OPS_616.md`](./AGENTS_POST_MERGE_OPS_616.md) — live wrap/window leftovers after !409–!413 ([#616](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/616))
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — #489; Protocol stays short “reference” labels, not TWAP vs CEX essays
- [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) — Global stats / fees / oracle chips are `flat` (#653); inline Δ% parentage is [#652](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652); visual grouping is [#667](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/667)
- [`AGENTS_FRONTEND_TRAILING_WINDOW.md`](./AGENTS_FRONTEND_TRAILING_WINDOW.md) — 24h/7d/30d volume is trailing, not calendar (#576)
- [`AGENTS_FRONTEND_TRADE_IDENTITY_LP.md`](./AGENTS_FRONTEND_TRADE_IDENTITY_LP.md) — Trade / Charts pair TVL chip reuses **P569** `protocol_pair_tvl` (#664); `/pool` column is #655
- Post-merge leftover: [#673](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/673) / `make verify-issue-673` / [`AGENTS_POST_MERGE_OPS_673.md`](./AGENTS_POST_MERGE_OPS_673.md)
