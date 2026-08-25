# Agent playbook: Hexxagon / Galaxy Station CW20s (GitLab #641)

Use when adding Terra Classic CW20 metadata for **Galaxy Station** via [hexxagon-io/chain-registry](https://github.com/hexxagon-io/chain-registry), or when someone asks why CL8Y / UST1 / wraps show as unnamed contracts there.

Parent catalog: [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639). This is **not** Keplr ([#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629)) and **not** Cosmostation ([#640](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/640) — that upstream is archived).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/listings/hexxagon/README.md](../docs/listings/hexxagon/README.md) | Invariants **H641-1–H641-8** |
| [`terra.fragment.json`](../docs/listings/hexxagon/terra.fragment.json) | Five rows to append (USTR omitted) |
| [docs/integrators.md § Hexxagon](../docs/integrators.md#hexxagon-cw20-gitlab-641) | Integrator summary |
| Upstream | [hexxagon-io/chain-registry](https://github.com/hexxagon-io/chain-registry) `cw20/tokens/mainnet/terra.js` |
| Live PR | [hexxagon-io/chain-registry#68](https://github.com/hexxagon-io/chain-registry/pull/68) |

## Invariants (H641-1–H641-8)

| ID | Rule |
|----|------|
| **H641-1** | File is `cw20/tokens/mainnet/terra.js` (Classic). Never phoenix / Terra 2. |
| **H641-2** | Permanent six pins/decimals match Keplr **K629-2**. Submit five; **USTR already live** as USTC Repeg. No gems / ALPHA / USTRIX / SpaceUSD / tax templates. |
| **H641-3** | Objects: `protocol`, `symbol`, `name`, `token`, `icon`, `decimals`. Optional `coinGeckoID` (Hexxagon spelling) only on CL8Y. Trailing commas. Append at end. |
| **H641-4** | `coinGeckoID` is `ceramicliberty-com` on CL8Y only. Do not invent a second id. |
| **H641-5** | UST1 is an **unstablecoin** (never `$1`). USTR is not a stablecoin and is verify-only here. |
| **H641-6** | Do not PR `terra-money/assets` — Hexxagon replaced that for Galaxy Station. |
| **H641-7** | Do not open the GitHub PR from this repo’s CI. |
| **H641-8** | This skill + README + fragment + `make verify-issue-641`. |

## Rules of thumb

1. **Do not resubmit USTR.** Live Hexxagon already lists it as USTC Repeg (`terra1vy3kc0…`, 18 decimals).
2. **Reuse Keplr decimals and gem exclude.** Close blocked if gems, wrong decimals, or the BSC CL8Y contract ship as Classic.
3. **Icons are in-repo GitLab raw URLs** under `tokenlist/images/` — do not invent a second logo host.
4. **This repo does not merge the GitHub PR.** The live append is [PR #68](https://github.com/hexxagon-io/chain-registry/pull/68). Chase that PR; do not open a duplicate.

## Verification

```bash
make verify-issue-641
```

No LocalTerra, indexer, or wallet. After upstream merge: Galaxy Station → Terra Classic → search **CL8Y**.

## Do not

- PR Cosmostation `chainlist` from here — that upstream is archived ([#640](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/640)).
- Reopen Keplr ([#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629)) or DeFiLlama ([#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631)).
- Treat `terra-money/assets` as the live Station list.

## Cross-links

- Parent catalog: [`AGENTS_LISTINGS.md`](./AGENTS_LISTINGS.md) · [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639)
- Keplr (same pins): [`AGENTS_KEPLR_CW20_REGISTRY.md`](./AGENTS_KEPLR_CW20_REGISTRY.md) · [#629](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/629)
- Cosmostation (archived upstream): [`AGENTS_COSMOSTATION.md`](./AGENTS_COSMOSTATION.md) · [#640](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/640)
- chain-registry / Leap: [#642](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/642)
- QA: [`docs/qa/issue-641/README.md`](../docs/qa/issue-641/README.md)
