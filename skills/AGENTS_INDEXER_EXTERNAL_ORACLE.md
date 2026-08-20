# Agent playbook: Indexer external USTC/LUNC/CEX-FDUSD oracle (GitLab #515 / #550 / #579 / #580)

Audience: third-party agents touching indexer USD reference prices, Protocol oracle UI, or integrator docs that mention `/api/v1/oracle/price`.

**Issue:** [GitLab **#515**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/515), [**#550**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550), [**#579**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/579) (CoinGecko User-Agent), [**#580**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/580)  
**Canonical runbook:** [`docs/runbooks/indexer-external-oracle.md`](../docs/runbooks/indexer-external-oracle.md)  
**Invariants table:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (rows **External oracle tickers #515**, **Protocol global stats #550**, **CoinGecko User-Agent #579**)

## Problem class

`GET /api/v1/oracle/price` historically returned **USTC/USD** (~0.005) with no ticker in the path. Integrators comparing to **LUNC/USD** (~0.00005) saw an exact **100×** overstatement and attributed KuCoin/MEXC “wrong” prices. Fix: ticker-scoped routes + catalog listing.

## Invariants (X1–X7)

| ID | Rule |
|----|------|
| **X1** | Bare `/api/v1/oracle/price` and `/history` return **catalog** `{ metadata, tickers }` only. |
| **X2** | Snapshots/history require `/price/{ticker}` or `/history/{ticker}` with `ustc` \| `lunc` \| `vfdusd`. |
| **X3** | Fetcher symbols must match ticker (USTC≠LUNC≠FDUSD CEX ids). Path `vfdusd` polls CEX FDUSD; logs/`display_name` are **FDUSD/USD**, never `vFDUSD/USD` ([#580](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/580)). |
| **X4** | Volume USD uses the **P522-Q catalog** ([#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548) / [#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556) hub USD for UST1/USTR). Overview `ustc_price_usd` stays the **USTC** handle. Do **not** convert DEX volume with vFDUSD/FDUSD. Never use `OracleTicker::Vfdusd` as `usd_per_human` for symbol `VFDUSD`. |
| **X5** | Advisory reference — never settlement authority. |
| **X6** | Non-finite f64 → safe BigDecimal before insert. |
| **X7** | Oracle `reqwest` client sends a stable descriptive User-Agent (`cl8y-dex-indexer/<CARGO_PKG_VERSION> (+https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic)`). CoinGecko **403 User-Agent required** is `MissingUserAgent`, **not** `RateLimited` (429). Do not spoof browsers, rotate UAs, put secrets in the UA, or delete the CoinGecko source. |

Do not confuse with on-chain **TWAP** ([`docs/twap-oracle.md`](../docs/twap-oracle.md)) or **UST1 window oracle** ([`AGENTS_UST1_WINDOW_UI.md`](./AGENTS_UST1_WINDOW_UI.md)).

## API contract (breaking v1)

```
GET /api/v1/oracle/price              → { metadata, tickers: ["ustc","lunc","vfdusd"] }
GET /api/v1/oracle/price/ustc         → { ticker, quote_asset, display_name, price_usd, sources }
GET /api/v1/oracle/price/lunc         → { ticker, quote_asset, display_name, price_usd, sources }
GET /api/v1/oracle/price/vfdusd       → { ticker: "vfdusd", quote_asset: "FDUSD", display_name: "FDUSD/USD", price_usd, sources }  # CEX FDUSD, not Terra vFDUSD
GET /api/v1/oracle/history            → catalog (same shape)
GET /api/v1/oracle/history/{ticker}   → { ticker, quote_asset, display_name, prices }
```

Frontend helpers: `getOraclePriceCatalog()`, `getOraclePrice(ticker?)` (default `ustc`), `getOracleHistory({ ticker?, ... })`.

## Do / don’t

- **Do** call `/price/ustc`, `/price/lunc`, or `/price/vfdusd` explicitly in new integrators.
- **Do** keep Protocol UI labeled **USTC / USD** when using the default ticker; LUNC and vFDUSD have their own chips ([#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550)). Protocol Venus “1 vFDUSD Price” is [#571](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/571) — not this CEX print.
- **Do** treat snapshot `quote_asset` / `display_name` as the CEX identity; path `vfdusd` is a stable URL, not “USD of 1 vFDUSD”.
- **Do** keep a stable descriptive User-Agent on the oracle HTTP client (`ORACLE_USER_AGENT` / `CARGO_PKG_VERSION` + repo URL). Wiremock tests must assert the header.
- **Don’t** restore a bare `/price` numeric response.
- **Don’t** use vFDUSD/FDUSD feeds for `volume_usd` conversion (LUNC-quoted swaps still use the LUNC feed via P522-Q).
- **Don’t** wire `OracleTicker::Vfdusd` into `quote_usd_kind` / P522-Q / hub USD as `usd_per_human` for symbol `VFDUSD`.
- **Don’t** log or document the CEX average as `vFDUSD/USD`.
- **Don’t** treat CoinGecko 429 as a hard outage (soft-fail; KuCoin/MEXC usually suffice). KuCoin is skipped for `vfdusd` (unlisted).
- **Don’t** ship the oracle HTTP client without a descriptive User-Agent ([#579](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/579)). CoinGecko 403 “add a User-Agent” is a **client bug**, not quota — map it to `MissingUserAgent`, not `RateLimited`.
- **Don’t** impersonate Chrome/Firefox/Keplr, rotate User-Agents to evade 429, scrape CoinGecko HTML, or raise poll frequency because 403 stopped.
- **Don’t** alias `/price/fdusd` to vFDUSD — unknown ticker stays **400**.
- **Don’t** hardcode vFDUSD or FDUSD as `$1`.
- **Don’t** use the LUNC ticker for the Charts/overview **USTC / USD** box.

## Regression checklist

1. `cd indexer && cargo test --lib oracle -- --quiet`
2. `cd indexer && cargo test --test api_oracle -- --test-threads=1`
3. Frontend mocks/types include `ticker` on price/history responses
4. `make verify-issue-515`
5. `make verify-issue-550` when touching Protocol UI or `vfdusd`
6. `make verify-issue-579` when touching the oracle HTTP client, CoinGecko fetch, or User-Agent (wiremock; no live CoinGecko)
7. `make verify-issue-580` when touching oracle display_name, catalog metadata, or P522-Q vFDUSD mapping

## Related

- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) — `/protocol` USD stats + ticker card ([#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550))
- [`AGENTS_LOCAL_POSTGRES_DEV.md`](./AGENTS_LOCAL_POSTGRES_DEV.md) — Postgres for integration tests
- [`docs/indexer-invariants.md`](../docs/indexer-invariants.md)
- [`docs/twap-oracle.md`](../docs/twap-oracle.md) — on-chain TWAP (different subsystem)
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — pair tape/candles convert quote-per-base to USD of 1 human base using these tickers ([#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522))
- [`AGENTS_FRONTEND_CHARTS_OVERVIEW.md`](./AGENTS_FRONTEND_CHARTS_OVERVIEW.md) — swap `volume_usd` ingest uses P522-Q + hub USD; overview USTC box stays this USTC feed ([#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548) / [#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556))
- [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) — DEX hub marks; `/oracle/price/ustr` stays 400
- [`AGENTS_FRONTEND_HUB_PNL.md`](./AGENTS_FRONTEND_HUB_PNL.md) — `/portfolio` + `/trader` realized P&amp;L USD uses hub snapshot, never CEX `/oracle/price/ustr` ([#560](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/560))
- GitLab [**#580**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/580) — CEX FDUSD identity under path `vfdusd`; `make verify-issue-580`
