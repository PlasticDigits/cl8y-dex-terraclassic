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
| ust1-window | `terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2` | **11566** | `VITE_UST1_WINDOW_ADDRESS` |

Approved window params (governance-updatable; always re-query on-chain): `fee_bps=100`, per-tx **1000** UST1 (`1000000000` raw @ 6dp), rolling 24h **10000** UST1 (`10000000000` raw). Observed `max_oracle_age_sec` on mainnet: **21600** (6h).

---

## Phase 3 — Treasury wrap ([ustr-cmm#5](https://gitlab.com/PlasticDigits2/ustr-cmm/-/work_items/5), DEX [#507](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/507))

| Role | Address | Code ID | Coolify / env key |
|------|---------|---------|-------------------|
| CMM treasury (wrap custody) | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` | — | `VITE_TREASURY_ADDRESS` |
| wrap-mapper | `terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2` | **11565** | `VITE_WRAP_MAPPER_ADDRESS` |
| cLUNC CW20 | `terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg` | — | `VITE_LUNC_C_TOKEN_ADDRESS` |
| cUSTC CW20 | `terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch` | — | `VITE_USTC_C_TOKEN_ADDRESS` |

**W2:** `VITE_TREASURY_ADDRESS` is the **ustr-cmm CMM treasury**, not the DEX factory fee/governance multisig `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7`.

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
| DEX governance / fee treasury | `terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7` | Factory `config.governance` |
| UST1/vFDUSD or UST1/cUSTC AMM | — | **Path B waiver** until seed inventory — [`../ust1-secondary-pair/PRODUCT_WAIVER.md`](../ust1-secondary-pair/PRODUCT_WAIVER.md) |

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
