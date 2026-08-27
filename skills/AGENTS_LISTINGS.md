# Agent playbook: listing venue catalog (GitLab #639)

Use when someone asks where to list CL8Y DEX, tokens, or wallet recognition — CoinGecko, CMC, Cosmostation, Hexxagon, chain-registry, GeckoTerminal, Trust Wallet, or “should we PR Coinhall / DexScreener / Leap?”

This is the **parent catalog**. It does **not** add indexer APIs.

**Issue:** [GitLab **#639**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639)  
**Catalog:** [`docs/listings/README.md`](../docs/listings/README.md) · [`docs/listings/catalog.json`](../docs/listings/catalog.json)  
**Form drafts:** [`docs/listings/forms/`](../docs/listings/forms/)  
**Verify:** `make verify-issue-639`

## Owned surfaces (do not reopen here)

| Surface | Issue | Skill |
|---------|-------|-------|
| DeFiLlama TVL / volume / fees | [#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631) | [`AGENTS_DEFILLAMA.md`](./AGENTS_DEFILLAMA.md) |
| Keplr Add Token CW20 pack | [#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629) | [`AGENTS_KEPLR_CW20_REGISTRY.md`](./AGENTS_KEPLR_CW20_REGISTRY.md) |
| `/cg/*` `/cmc/*` crawler shape | [#224](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/224) (timestamps) / [#685](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/685) (field truth) | [`docs/CG_CMC_COMPLIANCE.md`](../docs/CG_CMC_COMPLIANCE.md) · [`AGENTS_INDEXER_CG_CMC_LISTING.md`](./AGENTS_INDEXER_CG_CMC_LISTING.md) |

## Invariants (L639-1–L639-8)

| ID | Rule |
|----|------|
| **L639-1** | Catalog + go/no-go only. No new indexer endpoint. Children are **one venue per issue**. |
| **L639-2** | Permanent six CW20s only. Same pins/decimals as **K629-2** / **K629-3**. No gems / ALPHA / USTRIX / SpaceUSD / tax templates. |
| **L639-3** | Exchange forms use `https://indexer.dex.cl8y.com/cg/` and `/cmc/`. Never CoinGecko Pro v3. |
| **L639-4** | Llama = #631, Keplr = #629, crawler timestamps = #224, crawler field truth = #685. Do not reopen those here. |
| **L639-5** | UST1 is an **unstablecoin** (never `$1`). USTR is **not** a stablecoin. Do not invent a second CG id (`ceramicliberty-com`). |
| **L639-6** | Skip Coinhall, DexScreener, LuncScan Telegram, Token Terminal, CMC DexScan, Leap-own-repo, `terra-money/assets`. |
| **L639-7** | Do not open upstream PRs from CI. Forms may need a human captcha/login click. |
| **L639-8** | This skill + `docs/listings/README.md` + `catalog.json` + `make verify-issue-639`. |

## Rules of thumb

1. **Start a child, don’t dump venues into this MR.** Cosmostation pack exists but **upstream is archived** ([#640](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/640)) — do not open a GitHub PR. Live wallet PR is Hexxagon ([#641](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/641)), then chain-registry, then CG/CMC forms.
2. **Copy the Keplr/Llama skill shape** when a child starts: `docs/listings/<venue>/`, `skills/AGENTS_<VENUE>.md`, `make verify-issue-*`.
3. **Reuse Keplr decimals and gem exclude.** Close blocked if a child ships gems, wrong decimals, or the BSC CL8Y contract as Classic.
4. **Leap is chain-registry.** `suggestCW20Token` is per-user; official metadata is Cosmos Directory.
5. **GeckoTerminal is not Uniswap-V2 auto-detect.** Terra Classic is absent from GT networks. Non-EVM adapters are live at indexer `/gt/*` ([#646](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/646), `make verify-issue-646`). Do not point GT at `/cg/*`.
6. **Trust Wallet is paid.** Skip unless product asks.
7. **This repo does not merge GitHub listing PRs.** Export the pack, open the upstream PR, link it on the child issue.

## Verification

```bash
make verify-issue-639
```

No LocalTerra, indexer, or wallet. Child venue PRs and form submits are operator follow-up.

## Cross-links

- Keplr pack: [`AGENTS_KEPLR_CW20_REGISTRY.md`](./AGENTS_KEPLR_CW20_REGISTRY.md)
- DeFiLlama: [`AGENTS_DEFILLAMA.md`](./AGENTS_DEFILLAMA.md)
- Cosmostation (archived): [`AGENTS_COSMOSTATION.md`](./AGENTS_COSMOSTATION.md) · [#640](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/640)
- Hexxagon / Galaxy Station: [`AGENTS_HEXXAGON.md`](./AGENTS_HEXXAGON.md) · [#641](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/641)
- Retail gem hide: [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) (**#562**)
- Crawler field truth: [`AGENTS_INDEXER_CG_CMC_LISTING.md`](./AGENTS_INDEXER_CG_CMC_LISTING.md) · [#685](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/685)
- Integrators: [`docs/integrators.md`](../docs/integrators.md#listing-venue-catalog-gitlab-639)
- QA: [`docs/qa/issue-639/README.md`](../docs/qa/issue-639/README.md)
