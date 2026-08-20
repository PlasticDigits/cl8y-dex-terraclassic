# Indexer external USD oracle (GitLab #515 / #550)

Polled CEX/aggregator **reference** prices for TerraClassic **USTC/USD**, **LUNC/USD**, and **vFDUSD/USD** (CEX FDUSD). Distinct from on-chain pair **TWAP** ([`docs/twap-oracle.md`](../twap-oracle.md)) and the **UST1 window** rate.

## Why this exists

Volume USD denomination and Protocol UI need a stable off-chain USD reference. Historically `GET /api/v1/oracle/price` returned **USTC/USD** (~0.005) while some integrators treated the payload as **LUNC/USD** (~0.00005) — a ~100× overstatement ([#515](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/515)).

v1 now uses **ticker-scoped** paths (breaking change approved while non-economic).

## HTTP API

| Method | Path | Response |
|--------|------|----------|
| `GET` | `/api/v1/oracle/price` | Catalog: `{ metadata, tickers: ["ustc","lunc","vfdusd"] }` |
| `GET` | `/api/v1/oracle/price/{ticker}` | `{ ticker, price_usd, sources[] }` plus additive `venus` on **`vfdusd` only** ([#571](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/571)) |
| `GET` | `/api/v1/oracle/price/{ticker}/venus` | Venus snapshot `{ fdusd_per_vfdusd, source, fetched_at, vtoken }`. Ticker must be `vfdusd` else **400**. |
| `GET` | `/api/v1/oracle/history` | Same catalog as price |
| `GET` | `/api/v1/oracle/history/{ticker}` | `{ ticker, prices[] }` (average samples; `limit` capped 1000) |
| `GET` | `/api/v1/hub-prices` | DEX hub snapshot `{ metadata, tickers: ["custc","ust1","ustr"], prices[] }` ([#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556)) |
| `GET` | `/api/v1/hub-prices/{ticker}` | One DEX mark (`custc` \| `ust1` \| `ustr`). Unknown → **400**. |

Unknown CEX `{ticker}` (including `ustr` / `ust1` / `custc`) → **400**. DEX marks are **not** on `/oracle/price`.

### Tickers

| Ticker | Meaning | Example CEX symbols |
|--------|---------|---------------------|
| `ustc` | TerraClassic USTC per USD | KuCoin `USTC-USDT`, MEXC `USTCUSDT`, CoinGecko `terrausd` |
| `lunc` | TerraClassic LUNC per USD | KuCoin `LUNC-USDT`, MEXC `LUNCUSDT`, CoinGecko `terra-luna` |
| `vfdusd` | Wrapped FDUSD **CEX** reference (not a $1 peg). Protocol label: **FDUSD reference price**. | MEXC `FDUSDUSDT`, CoinGecko `first-digital-usd`; KuCoin **skipped** (unlisted). Path `fdusd` is **400** (no alias). |

Venus **1 vFDUSD → FDUSD** is **not** this CEX feed. Indexer poller: [`indexer/src/indexer/venus_vfdusd.rs`](../../indexer/src/indexer/venus_vfdusd.rs) (`exchangeRateStored` on Core Pool `0xC4eF4229FEc74Ccfe17B2bdeF7715fAC740BA0ba`). Stored in `venus_vfdusd_rates`, never mixed into `oracle_prices` USD. Skill: [`AGENTS_INDEXER_VENUS_VFDUSD.md`](../../skills/AGENTS_INDEXER_VENUS_VFDUSD.md).

### Sources

Each poll stores per-source rows plus an `average` row. In-memory cache serves `price_usd` on the ticker price endpoint. CoinGecko is polled on alternate ticks (rate-limit soft-fail).

## Storage

Table `oracle_prices(ticker, price_usd, source, fetched_at)` (migration `20260811000000_oracle_prices_multi_ticker.sql`). Replaces legacy `ustc_prices`.

Swap `volume_usd` uses the **P522-Q catalog** (GitLab [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548) / [#544](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/544) / [#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553) / [#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556)): USTC/cUSTC/`uusd` = this USTC feed; LUNC/cLUNC/`uluna` = the LUNC feed; UST1/USTR = **`hub_prices`** (DEX largest-liquidity marks). **USTR is set by the market, not a fixed `2.5 ×` USTC peg.** Unknown quotes stay NULL. Overview **`ustc_price_usd`** remains the USTC ticker only; hub fields are additive. DEX hub HTTP is `GET /api/v1/hub-prices` — **not** `/oracle/price/ustr` (400). Trader `total_volume_usd` is `SUM` of the same column. `/portfolio` + `/trader` header realized P&amp;L USD uses the same hub snapshot (**P560-1**, [#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560)) — never `$1` / `2.5×`.

## Invariants (X1–X6)

| ID | Rule |
|----|------|
| **X1** | `GET /api/v1/oracle/price` and `/history` are **catalogs only** — never a numeric price body. |
| **X2** | Price/history paths require an explicit ticker: `ustc`, `lunc`, or `vfdusd`. Unknown → **400**. |
| **X3** | Tickers use **distinct** CEX/CoinGecko symbols; never cross-wire LUNC/USTC/FDUSD ids. |
| **X4** | Indexer `volume_usd` uses the **P522-Q catalog** (USTC/LUNC oracles + hub USD for UST1/USTR, [#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556)). Overview `ustc_price_usd` stays the **USTC** feed. Do **not** convert DEX volume with vFDUSD/FDUSD or Venus. |
| **X5** | Feeds are **advisory** — not settlement; on-chain swaps use `max_spread` / `min_return` / deadlines. |
| **X6** | Non-finite `f64` → safe `BigDecimal` default before DB insert (existing oracle storage rule). |

## Code map

| Concern | Location |
|---------|----------|
| Poll loop + symbols | [`indexer/src/indexer/oracle.rs`](../../indexer/src/indexer/oracle.rs) |
| Venus vFDUSD redeem | [`indexer/src/indexer/venus_vfdusd.rs`](../../indexer/src/indexer/venus_vfdusd.rs) (#571) |
| HTTP handlers | [`indexer/src/api/oracle.rs`](../../indexer/src/api/oracle.rs) |
| DB queries | [`indexer/src/db/queries/oracle.rs`](../../indexer/src/db/queries/oracle.rs) |
| Frontend client | [`frontend-dapp/src/services/indexer/client.ts`](../../frontend-dapp/src/services/indexer/client.ts) |
| Agent playbook | [`skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md`](../../skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md) |
| Protocol UI | [`skills/AGENTS_FRONTEND_PROTOCOL_STATS.md`](../../skills/AGENTS_FRONTEND_PROTOCOL_STATS.md) (#550) |

## Regression

```bash
make verify-issue-515
make verify-issue-550
make verify-issue-571
# or: cd indexer && cargo test --lib oracle -- --quiet
#     cd indexer && cargo test --test api_oracle -- --test-threads=1
```
