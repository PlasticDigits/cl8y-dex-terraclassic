# Trade historical USD contract (#1512)

Use for Trade, Charts or Limits last-price USD, optional indexed USD, mixed decimals,
price-source fallback and display orientation. Issue: [#1512](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1512).

| ID | Invariant |
| --- | --- |
| TP1512-1 | `IndexerTrade.price` is always human factory asset_1 per asset_0. Missing `price_usd` does not select raw semantics. No implicit legacy branch remains. |
| TP1512-2 | Indexed `price_usd` is historical USD of one human factory asset_0. Display inversion divides by the matching human ratio once. Missing/nonpositive/malformed/nonfinite inputs and overflow/underflow return unavailable. |
| TP1512-3 | Callers use the selected pair, matching trade/pair addresses, validated 0..18 decimals and a nonfuture observation timestamp. An absent human ratio may use bounded uint128 raw amounts; malformed present ratios remain invalid. No decimal default is permitted. |
| TP1512-4 | Latest missing USD displays a stable em dash. No ticker/peg/USTR multiplier, stale last-good, candle-close or current-mark source may replace it. Current oracle fetching is unnecessary and removed from these callers. Historical indexed trades may be old; their own USD is not relabeled as a fresh oracle mark. |
| TP1512-5 | Candles and 24h stats retain their own historical USD/human fields and times. Tape Price stays human quote-per-base. Orientation persistence, execution/limit conversions, signing guards and LP accounting are independent and remain unchanged. |
| TP1512-6 | This display-only contract adds no chain/oracle reads and grants no signing permission. No live qualified-mark fallback is implemented. A future fallback needs exact identity, chain, original time, quality and separate provenance. |

See [frontend contract](../docs/frontend.md#trade-page-price-chart-invariants),
[P522](AGENTS_INDEXER_PAIR_PRICE_USD.md), [T524](AGENTS_FRONTEND_TRADE_PAIR_INVERT.md),
[chart guidance](AGENTS_FRONTEND_PRICE_CHART.md), and
[focused checklist/results](../docs/qa/issue-1512/README.md).
Run `make verify-issue-1512`; keep each test process under 60 seconds. Do not
claim deployment/browser/full issue acceptance from source unit checks.

For the additional deployed pair/orientation race and actual reload-persistence
cases, use the [bounded fixture checklist and coverage audit](../docs/qa/issue-1512/deployed-races.md).
Keep held responses actually fulfilled, own refetch clocks before navigation,
strip carried Charts price parameters before testing session persistence, and
retain failed attempts. These cases do not authorize signing or establish the
complete API/attack/funded matrix. Run only missing/relevant cases under 59s each;
the two document loads share one explicit total-request budget.
