# Agent playbook: Protocol vFDUSD Venus redeem rate (GitLab #571)

Audience: third-party agents touching `/protocol` vFDUSD tab copy, indexer BSC `eth_call`, or oracle JSON.

**Issue:** [GitLab **#571**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/571)  
**Related:** [#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550) Protocol card, [#515](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/515) CEX catalog, [#506](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506) UST1 window (different units)

## Problem class

The vFDUSD tab titled CEX **FDUSD/USD** as **vFDUSD / USD**. Bridged vFDUSD is a Venus vToken whose redeem value vs FDUSD accrues. Operators need both the CEX FDUSD reference and **how much FDUSD 1 vFDUSD redeems for**.

## Invariants (V571)

| ID | Rule |
|----|------|
| **V571-1** | vFDUSD tab CEX StatBox label is **FDUSD reference price**. Value = indexer CEX FDUSD/USD. Not `$1`, not Venus. |
| **V571-2** | Section **1 vFDUSD Price** (`protocol-oracle-vfdusd-venus`) shows human **FDUSD per 1 vFDUSD** from Venus `exchangeRateStored`. Source label **Venus**. Missing → `—`. |
| **V571-3** | Venus block only when `ticker === 'vfdusd'`. USTC/LUNC keep **Reference price**; no Venus fetch. |
| **V571-4** | CEX outage must not hide a healthy Venus row; Venus outage must not hide a healthy CEX row. |
| **V571-5** | Indexer owns BSC I/O (`eth_call` view). Browser / Vite CSP must **not** add BSC hosts. No `VITE_*` RPC URLs. |
| **V571-6** | Pin Core Pool vFDUSD `0xC4eF4229FEc74Ccfe17B2bdeF7715fAC740BA0ba`. Prefer `exchangeRateStored` (`0x182df0cd`). Do **not** send `exchangeRateCurrent` as a tx. |
| **V571-7** | Convert with on-chain vToken + underlying decimals. Display **1 human vFDUSD**, never raw-unit identity. Non-finite / zero / overflow → `—`. |
| **V571-8** | Do not substitute UST1 `effective_swap.oracle.rate` or CEX FDUSD/USD × Venus as either headline. Do not convert `volume_usd` with FDUSD/Venus (**X4**). No `/price/fdusd` alias. |
| **V571-9** | Persist Venus samples in `venus_vfdusd_rates`, **not** `oracle_prices` (wrong unit). History table stays CEX USD. |
| **V571-10** | Empty `BSC_RPC_URLS` skips live BSC (CI/LocalTerra). Soft-fail RPC; serve last cache. Never log RPC URLs/keys. |

## API

```
GET /api/v1/oracle/price/vfdusd         → { ticker, price_usd, sources, venus }
GET /api/v1/oracle/price/{ticker}/venus → Venus snapshot; ticker must be vfdusd else 400
```

`venus` is `null` on USTC/LUNC. Frontend Venus query is `getOracleVenusVfdusd()` with a hardcoded `vfdusd` path.

## Do / don’t

- **Do** keep tab chip **vFDUSD**. Heading on that tab is **vFDUSD** (no `/ USD`).
- **Do** mock BSC in tests (`wiremock`); do not require live BSC for `make verify-issue-571`.
- **Don’t** hardcode 1 vFDUSD = 1 FDUSD or `$1`.
- **Don’t** open BscScan / publicnode BSC from the dApp.

## Regression

```bash
make verify-issue-571
make verify-issue-550
make verify-issue-515
```

## Related

- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) (**P550-9** rewritten)
- [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md)
- [`docs/runbooks/indexer-external-oracle.md`](../docs/runbooks/indexer-external-oracle.md)
- [`docs/frontend.md`](../docs/frontend.md) § Protocol
