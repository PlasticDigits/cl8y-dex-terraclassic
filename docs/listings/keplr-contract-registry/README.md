# Keplr CW20 contract-registry pack (GitLab #629)

The permanent CL8Y ecosystem CW20 recognition pack for [chainapsis/keplr-contract-registry](https://github.com/chainapsis/keplr-contract-registry) supports Keplr Extension **Add Token** names, symbols, and logos. Upstream PR [#132](https://github.com/chainapsis/keplr-contract-registry/pull/132) merged on 2026-09-04.

This pack is **Job 1** (recognition). **Job 2** (native USD in Keplr) is documented below and is **not** solved by extra JSON fields we invent.

Agent playbook: [`skills/AGENTS_KEPLR_CW20_REGISTRY.md`](../../../skills/AGENTS_KEPLR_CW20_REGISTRY.md). Pins + builder: [`frontend-dapp/src/utils/keplrCw20Registry.ts`](../../../frontend-dapp/src/utils/keplrCw20Registry.ts). Verify the in-repo snapshot: `make verify-issue-629` (this does not query GitHub or open Keplr).

## Invariants (K629-1–K629-9)

| ID | Rule |
|----|------|
| **K629-1** | Chain folder is **`cosmos/columbus`** (`columbus-5`). Do **not** use `terra` or `phoenix` (Terra 2). `cosmos/columbus/base.json` already exists upstream (`chainId: columbus-5`). |
| **K629-2** | Register only the six permanent economic CW20s. **Exclude** soft-launch gems ([#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562) `COLUMBUS5_GEM_ADDRESSES`), ALPHA, USTRIX, SpaceUSD, and community-tax templates. |
| **K629-3** | Decimals are on-chain / dApp registry: **CL8Y 18**, **USTR 18**, UST1 / cLUNC / cUSTC / vFDUSD **6**. Never copy a stale `tokenlist.json` CL8Y `6` ([#476](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/476)). |
| **K629-4** | Each file is `{ contractAddress, imageUrl, metadata: { name, symbol, decimals } }`; `metadata` must match on-chain CW20 `token_info` (the upstream validator checks it). Optional **`coinGeckoId`** only when CoinGecko lists the economic asset (live columbus tokens such as MIR already use this field even though the upstream README omits it). **No** `price`, `priceUrl`, `oracle`, or `marketId`. |
| **K629-5** | Filename is the **full** contract address + `.json`. `imageUrl` is `…/images/columbus/<imageFile>`. Logos come from [`tokenlist/images/`](../../../tokenlist/images/). Keplr crops them to a circle. |
| **K629-6** | vFDUSD (UST1-window mint asset) is `terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3`, **6** decimals. |
| **K629-7** | Recognition ≠ price. CL8Y uses `coinGeckoId: ceramicliberty-com`. As verified 2026-09-24, CoinGecko still maps that id to the **BNB Chain** contract only. The Terra Classic platform request is tracked by [#644](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/644), child of [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639); it needs a human login/CAPTCHA. Other CMM assets have **no** CoinGecko id; omit the field. Offer indexer [`/api/v1/oracle/price`](../../runbooks/indexer-external-oracle.md) and [`/api/v1/hub-prices`](../../../skills/AGENTS_INDEXER_HUB_USD.md) as a source; do not encode them in the CW20 JSON. |
| **K629-8** | **USTR is already listed** upstream (`terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv.json`, name **USTC Repeg**, 18 decimals, `images/columbus/USTR.png`). Keep the in-repo copy for verify-only. Do **not** open a rename PR unless product asks. |
| **K629-9** | **Merged upstream is authoritative.** PR #132 merged on 2026-09-04. Its final CL8Y / cLUNC / cUSTC metadata was corrected to on-chain `token_info` (`CeramicLiberty.com CL8Y.com/bridge` / `CL8Y-cb`, `Wrapped LUNC`, `Wrapped USTC`). The in-repo snapshot still has pre-merge marketing strings for those rows; do not export it over upstream. Align it in a separate code change before any future submission. |

## Catalog

| Token | Decimals | Status | Contract |
|-------|----------|--------|----------|
| CL8Y | 18 | merged in PR #132; `ceramicliberty-com` | `terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3` |
| UST1 | 6 | merged in PR #132 | `terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72` |
| USTR | 18 | already registered; logo refreshed by PR #132 | `terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv` |
| cLUNC | 6 | merged in PR #132 | `terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg` |
| cUSTC | 6 | merged in PR #132 | `terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch` |
| vFDUSD | 6 | merged in PR #132 | `terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3` |

Local JSON snapshot for verification: [`cosmos/columbus/tokens/`](./cosmos/columbus/tokens/). It is not the current upstream submission after PR #132 merged.

## Export the local verification snapshot

The exporter below supplies files to the local validator. It is **not** a current upstream submission: PR #132's CI follow-up changed CL8Y, cLUNC, and cUSTC names to match chain `token_info`. Do not copy this snapshot over current upstream files until the local catalog has been aligned.

```bash
./scripts/qa/export-keplr-cw20-pack.sh /tmp/keplr-cl8y-pack
# writes cosmos/columbus/tokens/*.json and images/columbus/*.png
# default omits USTR (already upstream). Pass --include-registered to copy it too.
```

## Upstream status

[Keplr contract-registry PR #132](https://github.com/chainapsis/keplr-contract-registry/pull/132), opened 2026-08-25, merged to `chainapsis:main` on 2026-09-04 (`8cfd3f0`); all three reported checks passed. The five new CW20 files are present under `cosmos/columbus/tokens/`; USTR remains the existing **USTC Repeg** entry and its logo was refreshed in the PR. The merged files are the source of truth. A future change must start from current upstream main and match on-chain `token_info` before opening another PR.

## Job 2 — USD price (decision)

| Path | Decision |
|------|----------|
| Invent `price` / `priceUrl` / `oracle` on the CW20 JSON | **No.** Schema in the wild is `coinGeckoId` only (README still omits it). |
| Set `coinGeckoId: ceramicliberty-com` on CL8Y | **Yes** (Job 1 pack). Keplr can show CoinGecko’s USD if it keys by id. |
| CoinGecko Terra Classic platform for `ceramicliberty-com` | **Follow-up [#644](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/644)** (child of [#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639)). As verified 2026-09-24, the id maps to BNB Chain `0x8f45…` only. A human form request is queued in the cl8y-pm inbox. |
| coinGeckoId on UST1 / USTR / cLUNC / cUSTC / vFDUSD | **No** — no CoinGecko listing. |
| Offer indexer oracle / hub-prices to Keplr | **Documented offer only.** Not a substitute for CoinGecko or the #644 platform request. DeFiLlama [#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631) is a separate surface. |

Exchange crawlers (`/cg/*`, `/cmc/*`) are a different surface: [`docs/CG_CMC_COMPLIANCE.md`](../../CG_CMC_COMPLIANCE.md).

## Related

- Integrators: [`docs/integrators.md`](../../integrators.md#keplr-cw20-recognition-gitlab-629)
- Token list (logos + decimals): [`tokenlist/`](../../../tokenlist/)
- External oracle (not Keplr): [`docs/runbooks/indexer-external-oracle.md`](../../runbooks/indexer-external-oracle.md)
- QA: [`docs/qa/issue-629/README.md`](../../qa/issue-629/README.md)
- Other wallets / exchange forms (parent catalog): [`docs/listings/README.md`](../README.md) ([#639](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/639), [`skills/AGENTS_LISTINGS.md`](../../../skills/AGENTS_LISTINGS.md))
- CoinGecko Terra Classic platform request (Job 2): [#644](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/644) · [`coingecko-terra-classic-platform.md`](../forms/coingecko-terra-classic-platform.md)
