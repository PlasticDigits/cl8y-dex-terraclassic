# Agent playbook: Keplr CW20 recognition

Use when adding or changing Terra Classic CW20 metadata for **Keplr Add Token** (name, logo, optional `coinGeckoId`), or when someone asks why CL8Y / UST1 / wraps show as unnamed contracts in Keplr ([GitLab **#629**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629)).

This is **not** Keplr Ledger signing ([#567](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567)) and **not** CoinGecko/CMC **exchange** crawlers (`/cg/*`, `/cmc/*`).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/listings/keplr-contract-registry/README.md](../docs/listings/keplr-contract-registry/README.md) | Invariants **K629-1–K629-8**, catalog, submit steps |
| [`keplrCw20Registry.ts`](../frontend-dapp/src/utils/keplrCw20Registry.ts) | Pins + JSON builder |
| [`cosmos/columbus/tokens/`](../docs/listings/keplr-contract-registry/cosmos/columbus/tokens/) | Files to copy into the upstream PR |
| [`export-keplr-cw20-pack.sh`](../scripts/qa/export-keplr-cw20-pack.sh) | Drop-in tree (omits live USTR by default) |
| [docs/integrators.md § Keplr](../docs/integrators.md#keplr-cw20-recognition-gitlab-629) | Integrator summary |
| Upstream | [chainapsis/keplr-contract-registry](https://github.com/chainapsis/keplr-contract-registry) |

## Invariants (K629-1–K629-8)

1. **K629-1 Columbus folder** — files go under `cosmos/columbus/`. Never `terra` or `phoenix`.
2. **K629-2 Permanent six** — CL8Y, UST1, USTR, cLUNC, cUSTC, vFDUSD. No gems, ALPHA, USTRIX, SpaceUSD, or tax templates.
3. **K629-3 Decimals** — CL8Y **18**, USTR **18**, others **6**. Do not copy a stale tokenlist `6` for CL8Y.
4. **K629-4 Schema** — `contractAddress`, `imageUrl`, `metadata.{name,symbol,decimals}`. Optional `coinGeckoId` only when CoinGecko lists the asset. No `price` / `priceUrl` / `oracle` / `marketId`.
5. **K629-5 Paths** — filename = full address + `.json`. Logos from `tokenlist/images/` → `images/columbus/<file>`.
6. **K629-6 vFDUSD** — `terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3`, 6 decimals.
7. **K629-7 Price path** — recognition ≠ USD. CL8Y may set `ceramicliberty-com` (README omitted the field; live MIR already uses it). CoinGecko still maps that id to **BSC only** — request a Terra Classic platform add. Other CMM tokens have no CoinGecko id. Indexer oracle/hub-prices are an offer, not a Keplr field.
8. **K629-8 USTR live** — already registered as **USTC Repeg** (18 decimals). Verify-only unless product wants a rename. Docs + this playbook + `make verify-issue-629`.

## Rules of thumb

1. **Do not recreate `base.json`.** Upstream already has `cosmos/columbus/base.json` (`chainId: columbus-5`).
2. **Do not submit gems** to make Keplr “complete.” Retail hide ([#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562)) and Keplr registry stay aligned.
3. **Do not treat `/cg/*` as Keplr prices.** Exchange listing ≠ wallet token registry.
4. **Do not spam `suggestToken` for all six on every connect** unless product asks (six Keplr popups).
5. **This repo does not merge the GitHub PR.** Export the pack, open the upstream PR, link it on #629.

## Verification

```bash
make verify-issue-629
```

No LocalTerra, indexer, or wallet. After the upstream merge: Keplr → Terra Classic → Add Token → search CL8Y.

## Cross-links

- Ledger signing (different): [`AGENTS_FRONTEND_KEPLR_LEDGER.md`](./AGENTS_FRONTEND_KEPLR_LEDGER.md) ([#567](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567))
- CoinGecko User-Agent / CEX feeds: [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md) ([#515](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/515) / [#579](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/579))
- Hub USD (not Keplr): [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) ([#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556))
- DeFiLlama listing (related pricing coverage): [#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631)
- Other wallets / CG+CMC forms (parent catalog): [`AGENTS_LISTINGS.md`](./AGENTS_LISTINGS.md) ([#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639))
- Cosmostation / Mintscan (same pins, different repo): [`AGENTS_COSMOSTATION.md`](./AGENTS_COSMOSTATION.md) ([#640](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/640))
