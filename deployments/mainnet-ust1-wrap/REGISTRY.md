# Canonical mainnet address registry — UST1 + wrap (columbus-5)

**Issue:** [GitLab **#503**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503) (Phase 5 ops hardening)  
**Parent:** [#502](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502)  
**Network:** `columbus-5`  
**No secrets** — public contract addresses and Coolify env *keys* only.

When addresses change, update **this file first**, then mirror into Coolify, [`coolify.env.example`](./coolify.env.example), and the sibling repos noted below.

---

## Phase 2 — UST1 oracle window ([ust1-window#19](https://gitlab.com/PlasticDigits/ust1-window/-/issues/19), DEX [#506](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506))

| Role | Address | Code ID | Coolify / env key |
|------|---------|---------|-------------------|
| vFDUSD CW20 | `terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3` | — | `VITE_VFDUSD_TOKEN_ADDRESS` |
| UST1 CW20 | `terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72` | — | `VITE_UST1_TOKEN_ADDRESS` |
| ust1-oracle | `terra1fmht0t6svq3n24zx03nkfja0m40zhfyyxkdcvlrkl6u7gfe6aagq4gch8n` | **11568** | `VITE_UST1_ORACLE_ADDRESS` |
| ust1-window | `terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2` | **11566** | `VITE_UST1_WINDOW_ADDRESS` (dApp) · indexer `UST1_WINDOW_ADDRESS` ([#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614)) |

Approved window params (governance-updatable; always re-query on-chain): `fee_bps=100`, per-tx **1000** UST1 (`1000000000` raw @ 6dp), rolling 24h **10000** UST1 (`10000000000` raw). Observed `max_oracle_age_sec` on mainnet: **21600** (6h).

---

## Phase 3 — Treasury wrap ([ustr-cmm#5](https://gitlab.com/PlasticDigits2/ustr-cmm/-/work_items/5), DEX [#507](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/507))

| Role | Address | Code ID | Coolify / env key |
|------|---------|---------|-------------------|
| CMM treasury (wrap custody) | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` | — | `VITE_TREASURY_ADDRESS` |
| wrap-mapper | `terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2` | **11565** | `VITE_WRAP_MAPPER_ADDRESS` |
| cLUNC CW20 | `terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg` | — | `VITE_LUNC_C_TOKEN_ADDRESS` |
| cUSTC CW20 | `terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch` | — | `VITE_USTC_C_TOKEN_ADDRESS` |

**W2:** `VITE_TREASURY_ADDRESS` is the **ustr-cmm CMM treasury** (wrap custody). After fee-treasury rotation, factory/pair swap commissions use this same CMM address — not the DEX governance multisig `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7`.

Display symbols **cLUNC** / **cUSTC**; env keys may keep `LUNC_C` / `USTC_C` (**W3**). Wrap-mapper fees are **on-chain authoritative** (UI must query; do not hardcode).

| Config shape | When | Notes |
|--------------|------|-------|
| `{ fee_bps }` | Pre [ustr-cmm#9](https://gitlab.com/PlasticDigits2/ustr-cmm/-/work_items/9) migrate (code **11565** as of 2026-08-15) | Observed **200**. UI maps to both wrap and unwrap (truthful ~3.47% unwrap all-in). |
| `{ fee_wrap_bps, fee_unwrap_bps }` | Post migrate + `set_fees` | Product target **200 / 51** so unwrap 10 000 @ 1.5% tax ≈ **9 800**. `Config` drops `fee_bps`. |

Retune: `fee_unwrap_bps = round(10000 − 9800 / (1 − burn_tax_rate))` — [`skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md`](../../skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md) (**W15**, [#516](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/516)). DEX 2-of-3 must **not** sign mapper migrate / `SetFees`.

---

## Phase 4 — DEX wiring + secondary AMM ([#502](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502), [#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508))

| Role | Address | Notes |
|------|---------|-------|
| Factory | `terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea` | Soft-launch factory |
| Router | `terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw` | `SetWrapMapper` live (#502) |
| DEX governance (wasm admin) | `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` | Factory `config.governance` |
| DEX fee treasury (swaps + pair-creation) | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` | Factory `config.treasury` + each pair `GetFeeConfig.treasury` (CMM). Soft launch originally used the multisig — rotate with [`docs/runbooks/rotate-fee-treasury.md`](../../docs/runbooks/rotate-fee-treasury.md) |
| UST1/cUSTC AMM | `terra1ceprjsxp86ggftf5e38wwt34l83e5gq7penkdnv4wsatkwcs8v6qccw55f` | Factory pair; LP `UST1-CUST-LP` |
| UST1/USTR AMM | `terra16vxrhpvpcucu05y0nr862vf9hnqeh274uaff4s7hz4n0ea74006qf5hgqy` | Factory pair; LP `UST1-USTR-LP` |
| USTR CW20 | `terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv` | 18 decimals; extra minter includes DEX 2-of-3 |

---

## Community tax CW20 ([#592](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/592) / [#601](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/601))

| Role | Address / id | Coolify / env key |
|------|--------------|-------------------|
| Token template (factory-listed) | code **11611** · pin `9D33BF25…210EC2` | `VITE_COMMUNITY_TAX_CODE_ID` / `COMMUNITY_TAX_CODE_ID` |
| Token rotate (listed) | code **11619** · pin `63CB21D1…BAFA20` | listed 2026-08-24 tx [`B659D914…40CF`](https://finder.terraclassic.community/columbus-5/tx/B659D914CEAA045F05B5371F90669B79E34E7AE414537C6B4A8A58286BC640CF) height **30085794** |
| Launcher (canonical) | `terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze` (code **11620**, was 11614; wasm admin DEX 2-of-3) | `VITE_COMMUNITY_TOKEN_LAUNCHER` / `COMMUNITY_TOKEN_LAUNCHER` |
| AutoLP template (live bind pin) | code **11613** (launcher `GetConfig.autolp_code_id`) | bind later; not required for create |
| AutoLP rotate (stored, not listed) | code **11621** · pin `DAD413A3…8B76F1` | do not whitelist |
| CMM wasm admin / attested `cmm_governance` | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` | `CMM_GOVERNANCE_ADDR` (indexer); dApp admin-banner compare |

Factory whitelist after 2026-08-24: **`[6036, 8266, 10184, 11611, 11619]`**. Launcher instance is **11620**; `GetConfig.token_code_id` still **11611** / `autolp_code_id` **11613** until a new launcher store + `UpdateConfig`. Trace: [`../mainnet-soft-launch/deploy-trace.md`](../mainnet-soft-launch/deploy-trace.md). Listed REPORTs: [`../../cw20-codeid-audits/codeids/11611/REPORT.md`](../../cw20-codeid-audits/codeids/11611/REPORT.md), [`../../cw20-codeid-audits/codeids/11619/REPORT.md`](../../cw20-codeid-audits/codeids/11619/REPORT.md).

---

## Governance split (ops critical)

| Surface | On-chain `governance` (queried) | Signs pause / migrate |
|---------|----------------------------------|------------------------|
| ust1-window / wrap-mapper / CMM treasury | `terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l` | **ustr-cmm / UST1 stack** governance (not DEX multisig) |
| Factory / router / pairs | `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` | DEX soft-launch governance |

Do not assume the DEX multisig can pause wrap-mapper or the UST1 window.

---

## Cross-repo mirror checklist

| Repo | Doc to keep aligned |
|------|---------------------|
| **cl8y-dex-terraclassic** (this file) | Canonical |
| **ust1-window** | `docs/DEPLOYMENT.md` registry + oracle-service env |
| **ustr-cmm** | `docs/DEPLOYMENT.md` / `CONTRACTS.md` post-migrate wrap addresses |

---

## Related soft-launch traces

- Soft-launch DEX deploy: [`../mainnet-soft-launch/deploy-trace.md`](../mainnet-soft-launch/deploy-trace.md)
- Wrap Coolify-only template: [`../mainnet-soft-launch/wrap-enablement.env.example`](../mainnet-soft-launch/wrap-enablement.env.example)
