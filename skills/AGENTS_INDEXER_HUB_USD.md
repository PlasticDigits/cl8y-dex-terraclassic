# Agent playbook: DEX hub USD (cUSTC / LUNC / UST1 / USTR) (GitLab #556 / #570)

Audience: third-party agents touching pair `price_usd`, `volume_usd`, Protocol, or Charts **Price (USD)** for UST1/USTR/cUSTC/LUNC.

**Issue:** [GitLab **#556**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556) (ranking) · [**#570**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/570) (Protocol wrap identity + LUNC column)  
**Invariants:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (rows **DEX hub USD #556**, **DEX hub wrap identity #570**, **P522-Q**)  
**Verify:** `make verify-issue-556` · `make verify-issue-570`

## Problem class

P522-Q hardcoded **UST1 = $1** and **USTR = 2.5 × USTC**. Those are launch-seed pegs, not pool marks. Charts/Trade USD and `volume_usd` disagreed with the book.

## Invariants (H1–H16)

| ID | Rule |
|----|------|
| **H1** | `usd(cUSTC)` = `usd(uusd)` = #515 USTC CEX oracle. Oracle down → NULL, not `$1`. |
| **H2** | `usd(UST1)` from the **largest USD-TVL** factory pair whose legs are hub **cUSTC + UST1** (contract/denom), via humanized **reserves** (not last print, not `$1`). |
| **H3** | `usd(USTR)` from the **largest USD-TVL** factory pair vs already-priced cUSTC or UST1. **USTR is set by the market, not a fixed peg** — launch `2.5 ×` USTC is ops seed only. |
| **H4** | Pair `price_usd` / candles USD use hub quote USD **as-of ingest or current-bucket marks**. Do **not** rewrite historical tape from the live snapshot ([#568](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/568)). UI invert (#524 / #543) stays frontend `invertUsd`. |
| **H5** | `/protocol` shows a **DEX hub prices** card (`protocol-dex-hub-prices`) with cUSTC, **LUNC**, UST1, USTR ([#570](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/570)). CEX tabs stay `ustc` \| `lunc` \| `vfdusd`. |
| **H6** | `GET /api/v1/oracle/price/ustr` (and `ust1`, `custc`) remain **400**. Use `GET /api/v1/hub-prices`. `GET /hub-prices/lunc` is the DEX-card snapshot, distinct from CEX `/oracle/price/lunc`. |
| **H7** | `volume_usd` ingest uses hub USD for UST1/USTR quotes. Still **not** vFDUSD. |
| **H8** | Dust (`< $100` TVL default), stale reserves, unlisted pairs, and symbol-spoof natives cannot win ranking. Identity is **contract/denom**. |
| **H9** | This skill + invariants + `make verify-issue-556`. Wrap identity / LUNC column: `make verify-issue-570`. |
| **H10** | CG/CMC `last_price` stays human quote-per-base. |
| **H11** | Hub card column order: cUSTC, LUNC, UST1, USTR. Grid `sm:grid-cols-2 xl:grid-cols-4`. Four cells stay painted on hub 502 (USD `—`; wrap rows from env overlay). |
| **H12** | LUNC hub USD = #515 LUNC CEX wrap 1:1. Independent of USTC. Not a pool mark. |
| **H13** | cUSTC and LUNC columns show configured wrap CW20 `AddressRow` (cLUNC, not native `uluna`). |
| **H14** | UST1/USTR keep source-pair rows; cUSTC/LUNC `source_pair` stays null. |
| **H15** | `asset_address` on hub JSON is the configured wrap (`terra1` sanitize). Unknown / `clunc` path → 400. |
| **H16** | [`AGENTS_FRONTEND_PROTOCOL_HUB.md`](./AGENTS_FRONTEND_PROTOCOL_HUB.md) + `make verify-issue-570`. |

## Ranking

- TVL = human(`reserve_0`)×usd(asset_0) + human(`reserve_1`)×usd(asset_1). Not raw integers (18-dec vs 6-dec).
- Ties: max TVL, then lexicographic pair address.
- Bootstrap order is fixed (no UST1 from UST1/USTR using the old peg).
- Materialized in `hub_prices`; refresh on book-snapshot cadence + oracle tick. GET is O(1).

## Do / don’t

- **Do** read `GET /api/v1/hub-prices` (metadata says DEX, not CEX).
- **Do** keep ops LP seed `USTR_PER_USTC` in [`scripts/rebalance-mint-ust1-lp.sh`](../scripts/rebalance-mint-ust1-lp.sh) — sizing only.
- **Don’t** add `ustr` to `OracleTicker::ALL`.
- **Don’t** mix vFDUSD into hub USD or `volume_usd`.
- **Don’t** use CEX FDUSD (`OracleTicker::Vfdusd`) as `usd_per_human` for symbol `VFDUSD` ([#580](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/580)).
- **Don’t** scan `swap_events` on GET.
- **Don’t** rewrite historical `swap_events.price_usd` or candle USD from the live hub snapshot ([#568](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/568) **C568-1**). Idle charts use current-bucket mark bars (`trade_count = 0`).
- **Don’t** treat hub USD as settlement, TWAP, or the `/ust1` window rate.
- **Don’t** use hub USD for limit `validate_limit_order_price`.

## Env

| Variable | Default |
|----------|---------|
| `HUB_CUSTC_ADDRESS` | Columbus-5 tokenlist cUSTC |
| `HUB_CLUNC_ADDRESS` | Columbus-5 tokenlist cLUNC ([#570](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/570)) |
| `HUB_UST1_ADDRESS` | Columbus-5 tokenlist UST1 |
| `HUB_USTR_ADDRESS` | Columbus-5 tokenlist USTR |
| `HUB_USD_TVL_FLOOR` | `100` |

LocalTerra: set hub addresses to the deployed CW20s in `indexer/.env`.

## Related

- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — P522-1–5; P522-Q now hub
- [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md) — CEX catalog stays 3 tickers
- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) — page order includes DEX hub card; protocol **pool TVL** (#569) sums all priced factory pairs — not `hub_prices.tvl_usd` of one source pool
- [`AGENTS_FRONTEND_PROTOCOL_HUB.md`](./AGENTS_FRONTEND_PROTOCOL_HUB.md) — wrap AddressRows + LUNC column ([#570](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/570))
- [`AGENTS_REBALANCE_MINT_UST1_LP.md`](./AGENTS_REBALANCE_MINT_UST1_LP.md) — 2.5× seed is **not** a display oracle
- [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) — invert still UI-only
- [`AGENTS_FRONTEND_HUB_PNL.md`](./AGENTS_FRONTEND_HUB_PNL.md) — `/portfolio` + `/trader` realized P&amp;L USD from hub_prices ([#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560))
- [`AGENTS_INDEXER_CANDLE_USD_MARK.md`](./AGENTS_INDEXER_CANDLE_USD_MARK.md) — time-stamped candle USD; no as-of-now hub rewrite; idle mark bars ([#568](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/568))
