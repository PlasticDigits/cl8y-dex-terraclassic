# Agent playbook: Keplr CW20 recognition

Use when adding or changing Terra Classic CW20 metadata for **Keplr Add Token** (name, logo, optional `coinGeckoId`), or when someone asks why CL8Y / UST1 / wraps show as unnamed contracts in Keplr ([GitLab **#629**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629)).

This is **not** Keplr Ledger signing ([#567](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567)) and **not** CoinGecko/CMC **exchange** crawlers (`/cg/*`, `/cmc/*`).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/listings/keplr-contract-registry/README.md](../docs/listings/keplr-contract-registry/README.md) | Invariants **K629-1–K629-9**, catalog, merged upstream state |
| [`keplrCw20Registry.ts`](../frontend-dapp/src/utils/keplrCw20Registry.ts) | Pins + JSON builder |
| [`cosmos/columbus/tokens/`](../docs/listings/keplr-contract-registry/cosmos/columbus/tokens/) | Local verification snapshot; upstream PR #132 is already merged |
| [`export-keplr-cw20-pack.sh`](../scripts/qa/export-keplr-cw20-pack.sh) | Local snapshot exporter (omits live USTR by default; do not overwrite upstream) |
| [docs/integrators.md § Keplr](../docs/integrators.md#keplr-cw20-recognition-gitlab-629) | Integrator summary |
| Upstream | [chainapsis/keplr-contract-registry](https://github.com/chainapsis/keplr-contract-registry) |

## Invariants (K629-1–K629-8)

1. **K629-1 Columbus folder** — files go under `cosmos/columbus/`. Never `terra` or `phoenix`.
2. **K629-2 Permanent six** — CL8Y, UST1, USTR, cLUNC, cUSTC, vFDUSD. No gems, ALPHA, USTRIX, SpaceUSD, or tax templates.
3. **K629-3 Decimals** — CL8Y **18**, USTR **18**, others **6**. Do not copy a stale tokenlist `6` for CL8Y.
4. **K629-4 Schema** — `contractAddress`, `imageUrl`, `metadata.{name,symbol,decimals}`; metadata matches on-chain CW20 `token_info`. Optional `coinGeckoId` only when CoinGecko lists the asset. No `price` / `priceUrl` / `oracle` / `marketId`.
5. **K629-5 Paths** — filename = full address + `.json`. Logos from `tokenlist/images/` → `images/columbus/<file>`.
6. **K629-6 vFDUSD** — `terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3`, 6 decimals.
7. **K629-7 Price path** — recognition ≠ USD. CL8Y uses `ceramicliberty-com` (live registry entries already use `coinGeckoId` though the README omits it). As verified 2026-09-24, CoinGecko maps that id to **BNB Chain only**; the Terra Classic platform request is [#644](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/644), child of [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639). Other CMM tokens have no CoinGecko id. Indexer oracle/hub-prices are an offer, not a Keplr field.
8. **K629-8 USTR live** — already registered as **USTC Repeg** (18 decimals). Verify-only unless product wants a rename. Docs + this playbook + `make verify-issue-629`.
9. **K629-9 Merged source of truth** — upstream PR [#132](https://github.com/chainapsis/keplr-contract-registry/pull/132) merged 2026-09-04. Use current upstream JSON as authoritative: its CL8Y / cLUNC / cUSTC metadata was corrected to on-chain `token_info`. The in-repo pack still has pre-merge names for those rows; do not re-export it over upstream. Sync the local pack before any future submission.

## Rules of thumb

1. **Do not recreate `base.json`.** Upstream already has `cosmos/columbus/base.json` (`chainId: columbus-5`).
2. **Do not submit gems** to make Keplr “complete.” Retail hide ([#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562)) and Keplr registry stay aligned.
3. **Do not treat `/cg/*` as Keplr prices.** Exchange listing ≠ wallet token registry.
4. **Do not spam `suggestToken` for all six on every connect** unless product asks (six Keplr popups).
5. **This repo does not merge the GitHub PR.** PR #132 already merged. The exporter is for the local snapshot only; do not use its pre-merge metadata to overwrite upstream. Any future upstream change starts from current `chainapsis/main` and validates against on-chain `token_info`.

## Verification

```bash
make verify-issue-629
```

`make verify-issue-629` validates the in-repo snapshot only; it does not query GitHub or launch the Keplr extension. Upstream registry files for PR #132 are merged and authoritative. The post-merge Add Token UI check is a wallet-app task; route it through the [cl8y-pm inbox](https://pm.cl8y.com/inbox) using its [operator card format](https://git.cl8y.com/PlasticDigits/cl8y-pm/src/branch/main/docs/assigner-cards.md). Do not ask the operator to sign or submit a transaction. The CoinGecko price-path handoff is #644 and the form draft is [`docs/listings/forms/coingecko-terra-classic-platform.md`](../docs/listings/forms/coingecko-terra-classic-platform.md).

## Cross-links

- Ledger signing (different): [`AGENTS_FRONTEND_KEPLR_LEDGER.md`](./AGENTS_FRONTEND_KEPLR_LEDGER.md) ([#567](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/567))
- CoinGecko User-Agent / CEX feeds: [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md) ([#515](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/515) / [#579](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/579))
- Hub USD (not Keplr): [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) ([#556](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/556))
- DeFiLlama listing (related pricing coverage): [#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631)
- CoinGecko Terra Classic platform (Keplr Job 2): [#644](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/644)
- Other wallets / CG+CMC forms (parent catalog): [`AGENTS_LISTINGS.md`](./AGENTS_LISTINGS.md) ([#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639))
- Cosmostation / Mintscan (same pins, different repo): [`AGENTS_COSMOSTATION.md`](./AGENTS_COSMOSTATION.md) ([#640](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/640))
- Hexxagon / Galaxy Station (same pins, different repo): [`AGENTS_HEXXAGON.md`](./AGENTS_HEXXAGON.md) ([#641](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/641))
