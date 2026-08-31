# Agent playbook: newest-N candle GET + interval refit (GitLab #705)

Audience: third-party agents touching `GET /api/v1/pairs/{addr}/candles`, `PriceChart` interval chips, or lightweight-charts `fitContent` on `/charts` and `/trade`.

**Issue:** [GitLab **#705**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/705)  
**Invariants:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Candle newest-N read #705**), [`docs/frontend.md`](../docs/frontend.md) § Trade page — price chart invariants  
**Verify:** `make verify-issue-705`

## Problem class

After idle mark densification ([#568](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/568)), `1m`/`5m`/`15m` series are dense. `ORDER BY open_time ASC LIMIT 200` returned the **oldest** 200 bars in the 90-day window, so switching 1h → 15m plotted candles that stopped days in the past with empty time-scale to the right. The selected interval chip was a 14% blue wash.

Do **not** “fix” this by shrinking the 90-day window, disabling C568 marks, raising the HTTP `limit` cap above 1000, remounting the canvas on interval change, or calling `fitContent` on the 30s live refetch.

## Invariants (C705-1–C705-8)

| ID | Rule |
|----|------|
| **C705-1** | Default GET (no `from`/`to`) returns the **latest** `limit` bars in the 90-day window. Wire JSON is **oldest→newest** (`open_time` non-decreasing). Empty pair → `[]`. |
| **C705-2** | `limit` stays clamp **1–1000** (default 200). `limit=99999` → ≤ 1000 newest. Negative/zero → 1 row, never 500 ([#431](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/431)). Do not uncap to cover 90d of 1m. |
| **C705-3** | Explicit `from`/`to` still filter. Newest-N is **inside** that window (paging integrators must not see bars after `to`). Unparseable `from`/`to` keep the default window — no 500. |
| **C705-4** | Interval allowlist `1m\|5m\|15m\|1h\|4h\|1d\|1w` → else **400**. Unknown pair → **404**. Parameterized SQL only. Omitted interval defaults **1h**, still newest-N. |
| **C705-5** | Interval switch calls `timeScale().fitContent()` **once** after the new series `setData`. Canvas stays mounted ([#148](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/148)); pair switch still remounts via `key={pairAddress}`. Placeholder 1h rows must not refit; wait until first-bar time changes. |
| **C705-6** | Live 30s refetch must **not** `fitContent` ([#336](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/336)). Sliding newest-N (same interval, oldest bar dropped) must **not** refit. Indicator toggle / initial mount still may fit. |
| **C705-7** | Idle mark bars stay on GET (`trade_count = 0`, zero volume) and are eligible as the **last** bar of newest-N. Do not disable C568 to sparsify 15m. No client-side oracle stitch. |
| **C705-8** | Pressed interval uses `.price-chart-interval.tab-glass-active` (stronger `--blue`, not the 14% global wash). No gold fill, no `btn-primary`, no `*-neo`. Slippage / Pool Manage / Indicators keep generic `tab-glass-active`. `:focus-visible` ring stays. Light theme must still contrast. |

## Do / don’t

- **Do** wrap `get_candles` as `ORDER BY open_time DESC LIMIT n` then re-sort **ASC** (subquery). Never return DESC to the dApp.
- **Do** pass `interval` into `PriceChartLightweightCanvas` so refit is interval-scoped, not first-bar-time-only.
- **Do** keep dApp `getCandles(..., limit = 200)`. Newest-200 of 15m is ~50h of *recent* data.
- **Don’t** invent candles the indexer omitted.
- **Don’t** globally bump `.tab-glass-active` if that fights slippage chips — use the chart-interval modifier.
- **Don’t** treat Volume **0** on the last 15m bar as a fail when `trade_count = 0` (that is C568).

## Canonical code

| File | Role |
|------|------|
| `indexer/src/db/queries/candles.rs` | Newest-N subquery + ASC |
| `indexer/src/api/pairs.rs` | 90d default window, limit clamp, interval allowlist |
| `indexer/tests/api_candles_newest_n.rs` | Seed > limit; last `open_time` is newest; ASC; `from`/`to` |
| `frontend-dapp/src/components/charts/PriceChartLightweightCanvas.tsx` | Interval `fitContent` after `setData` |
| `frontend-dapp/src/components/charts/PriceChart.tsx` | Interval chips + `interval` prop |
| `frontend-dapp/src/index.css` | `.price-chart-interval.tab-glass-active` |

## Regression

```bash
make verify-issue-705
```

Indexer: `cd indexer && cargo test --test api_candles_newest_n -- --test-threads=1`.  
Frontend: `PriceChart.test.tsx`, `PriceChartLightweightCanvas.test.tsx` (fitContent split), `priceChartCandles.test.ts`, `designTokens.test.ts`.  
Marks still on GET: `cd indexer && cargo test --test candle_usd_mark get_candles_includes_mark_bars -- --test-threads=1` (full ladder: `make verify-issue-568`). `isolated_db` must not nest `lock_shared_test_db` after `setup_pool`.

## Related

- [`AGENTS_INDEXER_CANDLE_USD_MARK.md`](./AGENTS_INDEXER_CANDLE_USD_MARK.md) — idle marks densify 15m (**C568**); do not weaken
- [`AGENTS_FRONTEND_PRICE_CHART.md`](./AGENTS_FRONTEND_PRICE_CHART.md) — canvas lifecycle, #148 / #336
- [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md) — `tab-glass*` stays blue
- [`AGENTS_FRONTEND_USD_CANDLE_INVERT.md`](./AGENTS_FRONTEND_USD_CANDLE_INVERT.md) — newest-N rows still `invertUsd` per bar
