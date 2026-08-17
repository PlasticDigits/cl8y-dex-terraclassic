# Agent playbook: Indexer external USTC/LUNC oracle (GitLab #515)

Audience: third-party agents touching indexer USD reference prices, Protocol oracle UI, or integrator docs that mention `/api/v1/oracle/price`.

**Issue:** [GitLab **#515**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/515)  
**Canonical runbook:** [`docs/runbooks/indexer-external-oracle.md`](../docs/runbooks/indexer-external-oracle.md)  
**Invariants table:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **External oracle tickers #515**)

## Problem class

`GET /api/v1/oracle/price` historically returned **USTC/USD** (~0.005) with no ticker in the path. Integrators comparing to **LUNC/USD** (~0.00005) saw an exact **100×** overstatement and attributed KuCoin/MEXC “wrong” prices. Fix: ticker-scoped routes + catalog listing.

## Invariants (X1–X6)

| ID | Rule |
|----|------|
| **X1** | Bare `/api/v1/oracle/price` and `/history` return **catalog** `{ metadata, tickers }` only. |
| **X2** | Snapshots/history require `/price/{ticker}` or `/history/{ticker}` with `ustc` \| `lunc`. |
| **X3** | Fetcher symbols must match ticker (USTC≠LUNC CEX ids). |
| **X4** | Volume USD uses the **P522-Q catalog** ([#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548)). Overview `ustc_price_usd` stays the **USTC** handle. |
| **X5** | Advisory reference — never settlement authority. |
| **X6** | Non-finite f64 → safe BigDecimal before insert. |

Do not confuse with on-chain **TWAP** ([`docs/twap-oracle.md`](../docs/twap-oracle.md)) or **UST1 window oracle** ([`AGENTS_UST1_WINDOW_UI.md`](./AGENTS_UST1_WINDOW_UI.md)).

## API contract (breaking v1)

```
GET /api/v1/oracle/price              → { metadata, tickers: ["ustc","lunc"] }
GET /api/v1/oracle/price/ustc         → { ticker, price_usd, sources }
GET /api/v1/oracle/price/lunc         → { ticker, price_usd, sources }
GET /api/v1/oracle/history            → catalog (same shape)
GET /api/v1/oracle/history/{ticker}   → { ticker, prices }
```

Frontend helpers: `getOraclePriceCatalog()`, `getOraclePrice(ticker?)` (default `ustc`), `getOracleHistory({ ticker?, ... })`.

## Do / don’t

- **Do** call `/price/ustc` or `/price/lunc` explicitly in new integrators.
- **Do** keep Protocol UI labeled **USTC / USD** when using the default ticker.
- **Don’t** restore a bare `/price` numeric response.
- **Don’t** treat CoinGecko 429 as a hard outage (soft-fail; KuCoin/MEXC usually suffice).
- **Don’t** use the LUNC ticker for the Charts/overview **USTC / USD** box.

## Regression checklist

1. `cd indexer && cargo test --lib oracle -- --quiet`
2. `cd indexer && cargo test --test api_oracle -- --test-threads=1`
3. Frontend mocks/types include `ticker` on price/history responses
4. `make verify-issue-515`

## Related

- [`AGENTS_LOCAL_POSTGRES_DEV.md`](./AGENTS_LOCAL_POSTGRES_DEV.md) — Postgres for integration tests
- [`docs/indexer-invariants.md`](../docs/indexer-invariants.md)
- [`docs/twap-oracle.md`](../docs/twap-oracle.md) — on-chain TWAP (different subsystem)
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — pair tape/candles convert quote-per-base to USD of 1 human base using these tickers ([#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522))
- [`AGENTS_FRONTEND_CHARTS_OVERVIEW.md`](./AGENTS_FRONTEND_CHARTS_OVERVIEW.md) — swap `volume_usd` ingest uses P522-Q; overview USTC box stays this USTC feed ([#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548))
