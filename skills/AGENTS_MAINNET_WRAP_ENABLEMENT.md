# Agent playbook: mainnet wrap enablement (GitLab #507)

Use when enabling **native LUNC/USTC wrap** on columbus-5 **after** soft launch (post-SL5), setting Coolify `VITE_*` wrap env on **`https://dex.cl8y.com`**, or changing wrap fee display / simulation for **cLUNC** / **cUSTC**.

**Parent:** [GitLab **#502**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502) (router `SetWrapMapper` on-chain). **This issue:** [GitLab **#507**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/507) (Coolify env + Swap wrap UX: cLUNC/cUSTC symbols, on-chain fee display). Split wrap/unwrap fees: [#516](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/516).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`docs/runbooks/mainnet-soft-launch.md`](../docs/runbooks/mainnet-soft-launch.md) | SL1–SL7; post-SL5 wrap section |
| [`deployments/mainnet-soft-launch/wrap-enablement.env.example`](../deployments/mainnet-soft-launch/wrap-enablement.env.example) | Active Coolify template (four `VITE_*`) |
| [`deployments/mainnet-ust1-wrap/REGISTRY.md`](../deployments/mainnet-ust1-wrap/REGISTRY.md) | Canonical Phase 2–4 address registry (#503) |
| [`deployments/mainnet-soft-launch/deploy-trace.md`](../deployments/mainnet-soft-launch/deploy-trace.md) | Router ↔ wrap-mapper wiring (#502) + Phase 3 addresses |
| [`NATIVE_TOKEN_WRAPPING.md`](../NATIVE_TOKEN_WRAPPING.md) | Architecture + frontend integration |
| [`docs/qa-templates/wrap-unwrap-test-pass.md`](../docs/qa-templates/wrap-unwrap-test-pass.md) | Manual QA checklist |
| [`wrapMapper.ts`](../frontend-dapp/src/services/terraclassic/wrapMapper.ts) | `queryWrapMapperConfig`, `wrapMapperFeeBps(kind)`, `netAfterWrapMapperFee`, `wrapUnwrapFeeNote` |
| [`router.ts`](../frontend-dapp/src/services/terraclassic/router.ts) | `netCw20AfterNativeWrap`, `simulateNativeSwap` |
| [`constants.ts`](../frontend-dapp/src/utils/constants.ts) | `VITE_*` wrap addresses, `NATIVE_WRAPPED_PAIRS` |
| [`tokenRegistry.ts`](../frontend-dapp/src/utils/tokenRegistry.ts) | Display symbols **cLUNC** / **cUSTC** (env keys stay `VITE_LUNC_C_*` / `VITE_USTC_C_*`) |
| [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) | Wrap UX, fee note, safety CTA precedence; `getAllTokens` always surfaces LUNC/cLUNC/USTC/cUSTC when wrap env is set |
| [`WrapPage.tsx`](../frontend-dapp/src/pages/WrapPage.tsx) | Dedicated **More → Wrap** (`/wrap`) direct wrap/unwrap UI — chrome only (no educational/cross-nav/gas fluff; [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) **§9**) |
| [`PoolPage.tsx`](../frontend-dapp/src/pages/PoolPage.tsx) | Native-wrap provide / withdraw paths |

## Invariants (W1–W11)

| ID | Rule |
|----|------|
| **W1** | Coolify wrap `VITE_*` must point at **columbus-5 Phase 3** published addresses (see table below). Router already wired via governance `SetWrapMapper` ([#502](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502)). |
| **W2** | `VITE_TREASURY_ADDRESS` = **ustr-cmm CMM treasury** `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` (wrap custody). Same CMM address is the DEX swap-fee sink after treasury rotation — **not** the governance multisig `terra1zlmv2…`. |
| **W3** | UI display symbols are **cLUNC** / **cUSTC**; env var names may remain `VITE_LUNC_C_TOKEN_ADDRESS` / `VITE_USTC_C_TOKEN_ADDRESS`. |
| **W4** | UI, simulation, and execute paths use on-chain wrap-mapper fees: wrap → **`fee_wrap_bps`**, unwrap → **`fee_unwrap_bps`** (`net = amount − floor(amount × bps / 10_000)`). Never claim **1:1** when the matching fee > 0 **or when config LCD failed** (fail closed: disable submit / “Wrap fee unavailable”). Do not hardcode 200/51. Pre-migrate LCD may still return a single `fee_bps` — parser maps that to both sides. LocalTerra deploy default is often **50** unless changed. See **W12–W15** ([#516](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/516)). |
| **W5** | Soft-launch defaults script (**SL5**) must **not** silently deploy or enable economic wrap. Post-SL5 enablement is **Coolify env + frontend rebuild only** — see [`AGENTS_MAINNET_SOFT_LAUNCH.md`](./AGENTS_MAINNET_SOFT_LAUNCH.md). |
| **W6** | Swap CTA precedence: treasury mismatch → config unavailable → pause → blacklist → amount → rate limit. Fee display is separate from safety CTAs — [`AGENTS_FRONTEND_SWAP_SAFETY_CTA.md`](./AGENTS_FRONTEND_SWAP_SAFETY_CTA.md). Runtime-check `config.treasury` vs `VITE_TREASURY_ADDRESS` (W2). |
| **W7** | Retail wrap UI = title + asset/mode controls + live fee/rate-limit/pause + CTA. **Do not merge** “not an AMM”, “use Swap/UST1”, “Mapper Ready”, or always-on gas essays — depth belongs in docs (`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD` **§9**). Live wrap/unwrap fees / rate-limit / pause gates stay. Unwrap may use a **single** fee-line burn-tax disclosure + short exchange-deposit warning (**W10**/**W11**, [#512](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/512)) — not a permanent educational paragraph. |
| **W8** | Wrap / wrap-input mint quotes = mapper **`fee_wrap_bps`** only (`MsgExecuteContract` untaxed). [#512](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/512) / [#516](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/516). |
| **W9** | Unwrap / native-output **You Receive** = post-fee then InstantWithdraw burn tax; `routerMinReceiveBase` stays post-fee for **R3**. |
| **W10** | Unwrap fee note discloses burn tax on payout (one line). |
| **W11** | Unwrap UI warns against exchange deposit addresses as recipient. |

## Published columbus-5 addresses (Phase 3)

| Role | Address |
|------|---------|
| CMM treasury (`VITE_TREASURY_ADDRESS`) | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` |
| Wrap-mapper (`VITE_WRAP_MAPPER_ADDRESS`) | `terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2` |
| cLUNC (`VITE_LUNC_C_TOKEN_ADDRESS`) | `terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg` |
| cUSTC (`VITE_USTC_C_TOKEN_ADDRESS`) | `terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch` |
| Router (soft launch, unchanged) | `terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw` |

Wrap-mapper fees on mainnet: **on-chain authoritative**. Post-migrate target **`fee_wrap_bps=200` / `fee_unwrap_bps=51`** ([#516](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/516)); pre-migrate live may still be `{ fee_bps: 200 }`. Do not hardcode. Canonical pack: [`deployments/mainnet-ust1-wrap/REGISTRY.md`](../deployments/mainnet-ust1-wrap/REGISTRY.md).

## Coolify env keys (frontend build-args)

Set on Coolify for `docker/frontend/Dockerfile` rebuild after soft launch:

```bash
VITE_WRAP_MAPPER_ADDRESS=terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2
VITE_TREASURY_ADDRESS=terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2
VITE_LUNC_C_TOKEN_ADDRESS=terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg
VITE_USTC_C_TOKEN_ADDRESS=terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch
```

Copy-paste template: [`deployments/mainnet-soft-launch/wrap-enablement.env.example`](../deployments/mainnet-soft-launch/wrap-enablement.env.example).

**Do not** uncomment wrap keys in `deployments/mainnet-soft-launch/frontend.env.example` from the soft-launch deploy script — that file stays CW20-only; wrap lines are comment-only hints.

## Rules of thumb

1. Query `fee_wrap_bps` / `fee_unwrap_bps` via `queryWrapMapperConfig` (LCD) — do not hardcode 200/51. Transitional `{ fee_bps }` only when both split fields are absent.
2. Direct **wrap** quotes / `netCw20AfterNativeWrap` = **wrap** fee only (**W8** / **W12**). Direct **unwrap** / native-output = **unwrap** fee then burn tax (**W9**) — [`AGENTS_WRAP_UNWRAP_BURN_TAX.md`](./AGENTS_WRAP_UNWRAP_BURN_TAX.md), [`AGENTS_WRAP_MAPPER_SPLIT_FEES.md`](./AGENTS_WRAP_MAPPER_SPLIT_FEES.md).
3. Router `minimum_receive` on unwrap_output uses **post-fee pre-tax** (`routerMinReceiveBase`) — **R3**; display You Receive is post-tax.
4. Burn tax on InstantWithdraw is **additional** to `fee_unwrap_bps`. ≈2% all-in is the **51 bps + tax** retune, not gross-up (**W15**). Disclose on unwrap fee line (**W10**); keep `/wrap` free of permanent essays (**W7**).
5. Enabling wrap in Coolify does **not** require redeploying factory/router; it requires correct `VITE_*` + image rebuild. After #516, rebuild Coolify **before or with** wrap-mapper migrate.

## Verification

```bash
# Unit / lint (no chain)
make lint-frontend
make test-frontend
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
  src/services/terraclassic/__tests__/wrapMapper.test.ts \
  src/services/terraclassic/router.test.ts

# Soft-launch defaults must not ship active wrap keys
make test-mainnet-soft-launch-defaults
```

**Mainnet smoke** (wrap on production): operator QA per [`docs/qa-templates/wrap-unwrap-test-pass.md`](../docs/qa-templates/wrap-unwrap-test-pass.md) — not automated in CI.

## Related

- Split wrap/unwrap fees (#516): [`AGENTS_WRAP_MAPPER_SPLIT_FEES.md`](./AGENTS_WRAP_MAPPER_SPLIT_FEES.md)
- Unwrap burn tax incidence (#512): [`AGENTS_WRAP_UNWRAP_BURN_TAX.md`](./AGENTS_WRAP_UNWRAP_BURN_TAX.md)
- Native wrap amounts (#342/#512): [`AGENTS_NATIVE_WRAP_TAX.md`](./AGENTS_NATIVE_WRAP_TAX.md)
- Router `minimum_receive` / unwrap net: [`AGENTS_ROUTER_MINIMUM_RECEIVE.md`](./AGENTS_ROUTER_MINIMUM_RECEIVE.md)
- Swap pause / rate-limit CTAs: [`AGENTS_FRONTEND_SWAP_SAFETY_CTA.md`](./AGENTS_FRONTEND_SWAP_SAFETY_CTA.md)
- Soft launch (pre-wrap): [`AGENTS_MAINNET_SOFT_LAUNCH.md`](./AGENTS_MAINNET_SOFT_LAUNCH.md)
- Phase 5 ops / pause playbooks: [`AGENTS_UST1_WRAP_PRODUCTION_OPS.md`](./AGENTS_UST1_WRAP_PRODUCTION_OPS.md) ([#503](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503))
- Retail LUNC LP how-to (#531): [`AGENTS_FRONTEND_POOL_LP_HOWTO.md`](./AGENTS_FRONTEND_POOL_LP_HOWTO.md) — wrap is a **step** for native LUNC, not a new lecture on `/wrap`
- Architecture: [`NATIVE_TOKEN_WRAPPING.md`](../NATIVE_TOKEN_WRAPPING.md)
- Runbook: [`docs/runbooks/mainnet-soft-launch.md`](../docs/runbooks/mainnet-soft-launch.md)
