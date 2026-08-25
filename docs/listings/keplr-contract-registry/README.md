# Keplr CW20 contract-registry pack (GitLab #629)

Submit **permanent** CL8Y ecosystem CW20s to [chainapsis/keplr-contract-registry](https://github.com/chainapsis/keplr-contract-registry) so Keplr **Add Token** shows a name, symbol, and logo instead of a bare contract address.

This pack is **Job 1** (recognition). **Job 2** (native USD in Keplr) is documented below and is **not** solved by extra JSON fields we invent.

Agent playbook: [`skills/AGENTS_KEPLR_CW20_REGISTRY.md`](../../../skills/AGENTS_KEPLR_CW20_REGISTRY.md). Pins + builder: [`frontend-dapp/src/utils/keplrCw20Registry.ts`](../../../frontend-dapp/src/utils/keplrCw20Registry.ts). Verify: `make verify-issue-629`.

## Invariants (K629-1–K629-8)

| ID | Rule |
|----|------|
| **K629-1** | Chain folder is **`cosmos/columbus`** (`columbus-5`). Do **not** use `terra` or `phoenix` (Terra 2). `cosmos/columbus/base.json` already exists upstream (`chainId: columbus-5`). |
| **K629-2** | Register only the six permanent economic CW20s. **Exclude** soft-launch gems ([#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562) `COLUMBUS5_GEM_ADDRESSES`), ALPHA, USTRIX, SpaceUSD, and community-tax templates. |
| **K629-3** | Decimals are on-chain / dApp registry: **CL8Y 18**, **USTR 18**, UST1 / cLUNC / cUSTC / vFDUSD **6**. Never copy a stale `tokenlist.json` CL8Y `6` ([#476](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/476)). |
| **K629-4** | Each file is `{ contractAddress, imageUrl, metadata: { name, symbol, decimals } }`. Optional **`coinGeckoId`** only when CoinGecko lists the economic asset (live columbus tokens such as MIR already use this field even though the upstream README omits it). **No** `price`, `priceUrl`, `oracle`, or `marketId`. |
| **K629-5** | Filename is the **full** contract address + `.json`. `imageUrl` is `…/images/columbus/<imageFile>`. Logos come from [`tokenlist/images/`](../../../tokenlist/images/). Keplr crops them to a circle. |
| **K629-6** | vFDUSD (UST1-window mint asset) is `terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3`, **6** decimals. |
| **K629-7** | Recognition ≠ price. CL8Y may set `coinGeckoId: ceramicliberty-com`. CoinGecko still maps that id to the **BSC** contract only — request a Terra Classic platform add separately. Other CMM assets have **no** CoinGecko id; omit the field. Offer indexer [`/api/v1/oracle/price`](../../runbooks/indexer-external-oracle.md) and [`/api/v1/hub-prices`](../../../skills/AGENTS_INDEXER_HUB_USD.md) as a source; do not encode them in the CW20 JSON. |
| **K629-8** | **USTR is already listed** upstream (`terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv.json`, name **USTC Repeg**, 18 decimals, `images/columbus/USTR.png`). Keep the in-repo copy for verify-only. Do **not** open a rename PR unless product asks. |

## Catalog

| Token | Decimals | Status | Contract |
|-------|----------|--------|----------|
| CL8Y | 18 | submit (+ `ceramicliberty-com`) | `terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3` |
| UST1 | 6 | submit | `terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72` |
| USTR | 18 | already registered | `terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv` |
| cLUNC | 6 | submit | `terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg` |
| cUSTC | 6 | submit | `terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch` |
| vFDUSD | 6 | submit | `terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3` |

JSON ready to copy: [`cosmos/columbus/tokens/`](./cosmos/columbus/tokens/).

## Export a drop-in upstream tree

```bash
./scripts/qa/export-keplr-cw20-pack.sh /tmp/keplr-cl8y-pack
# writes cosmos/columbus/tokens/*.json and images/columbus/*.png
# default omits USTR (already upstream). Pass --include-registered to copy it too.
```

## Submit (operator, GitHub)

1. Fork [keplr-contract-registry](https://github.com/chainapsis/keplr-contract-registry).
2. Copy **submit** JSON files into `cosmos/columbus/tokens/` (do not recreate `base.json`).
3. Copy logos into `images/columbus/` (`CL8Y.png`, `UST1.png`, `CLUNC.png`, `CUSTC.png`, `VFDUSD.png`). Skip `USTR.png` unless replacing the live file.
4. Open a PR: *Add CL8Y ecosystem CW20s on columbus-5 (CL8Y, UST1, cLUNC, cUSTC, vFDUSD)*. Mention USTR is already listed. Link `https://dex.cl8y.com` and this issue.
5. After merge: Keplr Add Token → Terra Classic → search CL8Y / UST1 / cLUNC / cUSTC / vFDUSD.

Upstream PR (opened 2026-08-25): [chainapsis/keplr-contract-registry#132](https://github.com/chainapsis/keplr-contract-registry/pull/132). This repo cannot merge it. After merge, Keplr Add Token should show name + logo for the five new files.

## Job 2 — USD price (decision)

| Path | Decision |
|------|----------|
| Invent `price` / `priceUrl` / `oracle` on the CW20 JSON | **No.** Schema in the wild is `coinGeckoId` only (README still omits it). |
| Set `coinGeckoId: ceramicliberty-com` on CL8Y | **Yes** (Job 1 pack). Keplr can show CoinGecko’s USD if it keys by id. |
| CoinGecko Terra Classic platform for `ceramicliberty-com` | **Follow-up.** Today the id maps to BSC `0x8f45…` only. Request an additional Terra Classic CW20 platform. |
| coinGeckoId on UST1 / USTR / cLUNC / cUSTC / vFDUSD | **No** — no CoinGecko listing. |
| Offer indexer oracle / hub-prices to Keplr | **Documented offer only.** Not a substitute for CoinGecko. Related listing work: [#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631) DeFiLlama (pricing coverage). |

Exchange crawlers (`/cg/*`, `/cmc/*`) are a different surface: [`docs/CG_CMC_COMPLIANCE.md`](../../CG_CMC_COMPLIANCE.md).

## Related

- Integrators: [`docs/integrators.md`](../../integrators.md#keplr-cw20-recognition-gitlab-629)
- Token list (logos + decimals): [`tokenlist/`](../../../tokenlist/)
- External oracle (not Keplr): [`docs/runbooks/indexer-external-oracle.md`](../../runbooks/indexer-external-oracle.md)
- QA: [`docs/qa/issue-629/README.md`](../../qa/issue-629/README.md)
