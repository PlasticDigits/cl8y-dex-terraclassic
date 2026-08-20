# Indexer external USD oracle (GitLab #515 / #550 / #579 / #580)

Polled CEX/aggregator **reference** prices for TerraClassic **USTC/USD**, **LUNC/USD**, and CEX **FDUSD/USD** (HTTP/DB path `vfdusd`). Distinct from on-chain pair **TWAP** ([`docs/twap-oracle.md`](../twap-oracle.md)) and the **UST1 window** rate.

**Identity (#580):** path `vfdusd` stores **CEX FDUSD/USD** (MEXC `FDUSDUSDT`, CoinGecko `first-digital-usd`). It is **not** USD of Terra CW20 **vFDUSD** (Venus bridged). Operator logs use `display_name` **FDUSD/USD**, never `vFDUSD/USD`. Additive JSON `quote_asset=FDUSD` / `display_name=FDUSD/USD`. Path `fdusd` stays **400**. Protocol Venus “1 vFDUSD Price” is [#571](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/571) — not this feed.

## Why this exists

Volume USD denomination and Protocol UI need a stable off-chain USD reference. Historically `GET /api/v1/oracle/price` returned **USTC/USD** (~0.005) while some integrators treated the payload as **LUNC/USD** (~0.00005) — a ~100× overstatement ([#515](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/515)).

v1 now uses **ticker-scoped** paths (breaking change approved while non-economic).

## HTTP API

| Method | Path | Response |
|--------|------|----------|
| `GET` | `/api/v1/oracle/price` | Catalog: `{ metadata, tickers: ["ustc","lunc","vfdusd"] }` |
| `GET` | `/api/v1/oracle/price/{ticker}` | `{ ticker, quote_asset, display_name, price_usd, sources[] }` |
| `GET` | `/api/v1/oracle/history` | Same catalog as price |
| `GET` | `/api/v1/oracle/history/{ticker}` | `{ ticker, quote_asset, display_name, prices[] }` (average samples; `limit` capped 1000) |
| `GET` | `/api/v1/hub-prices` | DEX hub snapshot `{ metadata, tickers: ["custc","ust1","ustr"], prices[] }` ([#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556)) |
| `GET` | `/api/v1/hub-prices/{ticker}` | One DEX mark (`custc` \| `ust1` \| `ustr`). Unknown → **400**. |

Unknown CEX `{ticker}` (including `ustr` / `ust1` / `custc`) → **400**. DEX marks are **not** on `/oracle/price`.

### Tickers

| Ticker | Meaning | Example CEX symbols |
|--------|---------|---------------------|
| `ustc` | TerraClassic USTC per USD | KuCoin `USTC-USDT`, MEXC `USTCUSDT`, CoinGecko `terrausd` |
| `lunc` | TerraClassic LUNC per USD | KuCoin `LUNC-USDT`, MEXC `LUNCUSDT`, CoinGecko `terra-luna` |
| `vfdusd` | CEX FDUSD/USD reference stored under path `vfdusd` — **not** Terra CW20 vFDUSD | MEXC `FDUSDUSDT`, CoinGecko `first-digital-usd`; KuCoin **skipped** (unlisted). Path `fdusd` is **400** (no alias). JSON `quote_asset=FDUSD`, `display_name=FDUSD/USD`. |

### Sources

Each poll stores per-source rows plus an `average` row. In-memory cache serves `price_usd` on the ticker price endpoint. CoinGecko is polled on alternate ticks (rate-limit soft-fail).

The oracle HTTP client sends a **stable descriptive User-Agent** (`cl8y-dex-indexer/<version> (+https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic)`) on every KuCoin / MEXC / CoinGecko request ([#579](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/579)). CoinGecko’s free API returns **HTTP 403** (`Please add a descriptive User-Agent`) if this header is missing or empty — that is a **client misconfiguration**, not quota. Distinguish:

| Status | Meaning | Log / error |
|--------|---------|-------------|
| **429** (or JSON `error_code: 429`) | Rate limited | `RateLimited` — debug; skip this tick |
| **403** body asks for User-Agent | Missing descriptive UA | `MissingUserAgent` — error once, then debug; **not** 429 |
| **403** other (ban / key) | Soft-fail | Parse/HTTP warn; truncated body (~120 chars) |

Do **not** impersonate browsers, rotate User-Agents, or drop the CoinGecko source to “fix” 403. KuCoin/MEXC still average when CoinGecko is down (vFDUSD has no KuCoin pair).

## Storage

Table `oracle_prices(ticker, price_usd, source, fetched_at)` (migration `20260811000000_oracle_prices_multi_ticker.sql`). Replaces legacy `ustc_prices`.

Swap `volume_usd` uses the **P522-Q catalog** (GitLab [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548) / [#544](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/544) / [#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553) / [#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556)): USTC/cUSTC/`uusd` = this USTC feed; LUNC/cLUNC/`uluna` = the LUNC feed; UST1/USTR = **`hub_prices`** (DEX largest-liquidity marks). **USTR is set by the market, not a fixed `2.5 ×` USTC peg.** Unknown quotes stay NULL. Overview **`ustc_price_usd`** remains the USTC ticker only; hub fields are additive. DEX hub HTTP is `GET /api/v1/hub-prices` — **not** `/oracle/price/ustr` (400). Trader `total_volume_usd` is `SUM` of the same column. `/portfolio` + `/trader` header realized P&amp;L USD uses the same hub snapshot (**P560-1**, [#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560)) — never `$1` / `2.5×`.

## Invariants (X1–X7)

| ID | Rule |
|----|------|
| **X1** | `GET /api/v1/oracle/price` and `/history` are **catalogs only** — never a numeric price body. |
| **X2** | Price/history paths require an explicit ticker: `ustc`, `lunc`, or `vfdusd`. Unknown → **400**. |
| **X3** | Tickers use **distinct** CEX/CoinGecko symbols; never cross-wire LUNC/USTC/FDUSD ids. Path `vfdusd` polls CEX FDUSD; logs/`display_name` are **FDUSD/USD**, not `vFDUSD/USD` ([#580](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/580)). |
| **X4** | Indexer `volume_usd` uses the **P522-Q catalog** (USTC/LUNC oracles + hub USD for UST1/USTR, [#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556)). Overview `ustc_price_usd` stays the **USTC** feed. Do **not** convert DEX volume with vFDUSD/FDUSD. Never use `OracleTicker::Vfdusd` as `usd_per_human` for symbol `VFDUSD`. |
| **X5** | Feeds are **advisory** — not settlement; on-chain swaps use `max_spread` / `min_return` / deadlines. |
| **X6** | Non-finite `f64` → safe `BigDecimal` default before DB insert (existing oracle storage rule). |
| **X7** | Oracle HTTP client sends a stable, non-browser User-Agent identifying this indexer + repo ([#579](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/579)). CoinGecko 403 User-Agent-missing is **not** `RateLimited`. Soft-fail: one source down still averages the rest. |

## Code map

| Concern | Location |
|---------|----------|
| Poll loop + symbols | [`indexer/src/indexer/oracle.rs`](../../indexer/src/indexer/oracle.rs) |
| HTTP handlers | [`indexer/src/api/oracle.rs`](../../indexer/src/api/oracle.rs) |
| DB queries | [`indexer/src/db/queries/oracle.rs`](../../indexer/src/db/queries/oracle.rs) |
| Frontend client | [`frontend-dapp/src/services/indexer/client.ts`](../../frontend-dapp/src/services/indexer/client.ts) |
| Agent playbook | [`skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md`](../../skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md) |
| Protocol UI | [`skills/AGENTS_FRONTEND_PROTOCOL_STATS.md`](../../skills/AGENTS_FRONTEND_PROTOCOL_STATS.md) (#550) |

## Regression

```bash
make verify-issue-579   # User-Agent + 403 vs 429 (no live CoinGecko; --lib oracle)
make verify-issue-515
make verify-issue-550
make verify-issue-580
# or: cd indexer && cargo test --lib oracle -- --quiet
#     cd indexer && cargo test --test api_oracle -- --test-threads=1
```
