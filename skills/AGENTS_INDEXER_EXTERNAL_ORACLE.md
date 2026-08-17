# Agent playbook: Indexer external USTC/LUNC/vFDUSD oracle (GitLab #515 / #550)

Audience: third-party agents touching indexer USD reference prices, Protocol oracle UI, or integrator docs that mention `/api/v1/oracle/price`.

**Issue:** [GitLab **#515**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/515), [**#550**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550)  
**Canonical runbook:** [`docs/runbooks/indexer-external-oracle.md`](../docs/runbooks/indexer-external-oracle.md)  
**Invariants table:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (rows **External oracle tickers #515**, **Protocol global stats #550**)

## Problem class

`GET /api/v1/oracle/price` historically returned **USTC/USD** (~0.005) with no ticker in the path. Integrators comparing to **LUNC/USD** (~0.00005) saw an exact **100×** overstatement and attributed KuCoin/MEXC “wrong” prices. Fix: ticker-scoped routes + catalog listing.

## Invariants (X1–X6)

| ID | Rule |
|----|------|
| **X1** | Bare `/api/v1/oracle/price` and `/history` return **catalog** `{ metadata, tickers }` only. |
| **X2** | Snapshots/history require `/price/{ticker}` or `/history/{ticker}` with `ustc` \| `lunc` \| `vfdusd`. |
| **X3** | Fetcher symbols must match ticker (USTC≠LUNC≠FDUSD CEX ids). |
| **X4** | Volume USD / overview `ustc_price_usd` stay on the **USTC** handle. |
| **X5** | Advisory reference — never settlement authority. |
| **X6** | Non-finite f64 → safe BigDecimal before insert. |

Do not confuse with on-chain **TWAP** ([`docs/twap-oracle.md`](../docs/twap-oracle.md)) or **UST1 window oracle** ([`AGENTS_UST1_WINDOW_UI.md`](./AGENTS_UST1_WINDOW_UI.md)).

## API contract (breaking v1)

```
GET /api/v1/oracle/price              → { metadata, tickers: ["ustc","lunc","vfdusd"] }
GET /api/v1/oracle/price/ustc         → { ticker, price_usd, sources }
GET /api/v1/oracle/price/lunc         → { ticker, price_usd, sources }
GET /api/v1/oracle/price/vfdusd       → { ticker, price_usd, sources }  # CEX FDUSD, labeled vFDUSD
GET /api/v1/oracle/history            → catalog (same shape)
GET /api/v1/oracle/history/{ticker}   → { ticker, prices }
```

Frontend helpers: `getOraclePriceCatalog()`, `getOraclePrice(ticker?)` (default `ustc`), `getOracleHistory({ ticker?, ... })`.

## Do / don’t

- **Do** call `/price/ustc`, `/price/lunc`, or `/price/vfdusd` explicitly in new integrators.
- **Do** keep Protocol UI labeled **USTC / USD** when using the default ticker; LUNC and vFDUSD have their own chips ([#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550)).
- **Don’t** restore a bare `/price` numeric response.
- **Don’t** use LUNC or vFDUSD feeds for `volume_usd` conversion.
- **Don’t** treat CoinGecko 429 as a hard outage (soft-fail; KuCoin/MEXC usually suffice). KuCoin is skipped for `vfdusd` (unlisted).
- **Don’t** alias `/price/fdusd` to vFDUSD — unknown ticker stays **400**.
- **Don’t** hardcode vFDUSD as `$1`.

## Regression checklist

1. `cd indexer && cargo test --lib oracle -- --quiet`
2. `cd indexer && cargo test --test api_oracle -- --test-threads=1`
3. Frontend mocks/types include `ticker` on price/history responses
4. `make verify-issue-515`
5. `make verify-issue-550` when touching Protocol UI or `vfdusd`

## Related

- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) — `/protocol` USD stats + ticker card ([#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550))
- [`AGENTS_LOCAL_POSTGRES_DEV.md`](./AGENTS_LOCAL_POSTGRES_DEV.md) — Postgres for integration tests
- [`docs/indexer-invariants.md`](../docs/indexer-invariants.md)
- [`docs/twap-oracle.md`](../docs/twap-oracle.md) — on-chain TWAP (different subsystem)
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — pair tape/candles convert quote-per-base to USD of 1 human base using these tickers ([#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522))
