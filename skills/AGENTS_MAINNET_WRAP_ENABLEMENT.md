# Agent playbook: mainnet wrap enablement (GitLab #507)

Use when enabling **native LUNC/USTC wrap** on columbus-5 **after** soft launch (post-SL5), setting Coolify `VITE_*` wrap env on **`https://dex.cl8y.com`**, or changing wrap fee display / simulation for **cLUNC** / **cUSTC**.

**Parent:** [GitLab **#502**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502) (router `SetWrapMapper` on-chain). **This issue:** [GitLab **#507**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/507) (Coolify env + Swap wrap UX: cLUNC/cUSTC symbols, `fee_bps` display).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [`docs/runbooks/mainnet-soft-launch.md`](../docs/runbooks/mainnet-soft-launch.md) | SL1–SL7; post-SL5 wrap section |
| [`deployments/mainnet-soft-launch/wrap-enablement.env.example`](../deployments/mainnet-soft-launch/wrap-enablement.env.example) | Active Coolify template (four `VITE_*`) |
| [`deployments/mainnet-soft-launch/deploy-trace.md`](../deployments/mainnet-soft-launch/deploy-trace.md) | Router ↔ wrap-mapper wiring (#502) + Phase 3 addresses |
| [`NATIVE_TOKEN_WRAPPING.md`](../NATIVE_TOKEN_WRAPPING.md) | Architecture + frontend integration |
| [`docs/qa-templates/wrap-unwrap-test-pass.md`](../docs/qa-templates/wrap-unwrap-test-pass.md) | Manual QA checklist |
| [`wrapMapper.ts`](../frontend-dapp/src/services/terraclassic/wrapMapper.ts) | `queryWrapMapperConfig`, `netAfterWrapMapperFee`, `wrapUnwrapFeeNote` |
| [`router.ts`](../frontend-dapp/src/services/terraclassic/router.ts) | `netCw20AfterNativeWrap`, `simulateNativeSwap` |
| [`constants.ts`](../frontend-dapp/src/utils/constants.ts) | `VITE_*` wrap addresses, `NATIVE_WRAPPED_PAIRS` |
| [`tokenRegistry.ts`](../frontend-dapp/src/utils/tokenRegistry.ts) | Display symbols **cLUNC** / **cUSTC** (env keys stay `VITE_LUNC_C_*` / `VITE_USTC_C_*`) |
| [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) | Wrap UX, fee note, safety CTA precedence; `getAllTokens` always surfaces LUNC/cLUNC/USTC/cUSTC when wrap env is set |
| [`WrapPage.tsx`](../frontend-dapp/src/pages/WrapPage.tsx) | Dedicated **More → Wrap** (`/wrap`) direct wrap/unwrap UI |
| [`PoolPage.tsx`](../frontend-dapp/src/pages/PoolPage.tsx) | Native-wrap provide / withdraw paths |

## Invariants (W1–W6)

| ID | Rule |
|----|------|
| **W1** | Coolify wrap `VITE_*` must point at **columbus-5 Phase 3** published addresses (see table below). Router already wired via governance `SetWrapMapper` ([#502](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502)). |
| **W2** | `VITE_TREASURY_ADDRESS` = **ustr-cmm CMM treasury** `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` — **not** the DEX factory fee treasury / governance multisig `terra1zlmv2…`. |
| **W3** | UI display symbols are **cLUNC** / **cUSTC**; env var names may remain `VITE_LUNC_C_TOKEN_ADDRESS` / `VITE_USTC_C_TOKEN_ADDRESS`. |
| **W4** | UI, simulation, and execute paths use on-chain wrap-mapper **`fee_bps`**: `net = amount − floor(amount × fee_bps / 10_000)`. Never claim **1:1** when `fee_bps > 0` **or when config LCD failed** (fail closed: disable submit / “Wrap fee unavailable”). Mainnet Phase 3: **`fee_bps = 100`** (1%). LocalTerra deploy default is often **50** unless changed. |
| **W5** | Soft-launch defaults script (**SL5**) must **not** silently deploy or enable economic wrap. Post-SL5 enablement is **Coolify env + frontend rebuild only** — see [`AGENTS_MAINNET_SOFT_LAUNCH.md`](./AGENTS_MAINNET_SOFT_LAUNCH.md). |
| **W6** | Swap CTA precedence: treasury mismatch → config unavailable → pause → blacklist → amount → rate limit. Fee display is separate from safety CTAs — [`AGENTS_FRONTEND_SWAP_SAFETY_CTA.md`](./AGENTS_FRONTEND_SWAP_SAFETY_CTA.md). Runtime-check `config.treasury` vs `VITE_TREASURY_ADDRESS` (W2). |

## Published columbus-5 addresses (Phase 3)

| Role | Address |
|------|---------|
| CMM treasury (`VITE_TREASURY_ADDRESS`) | `terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2` |
| Wrap-mapper (`VITE_WRAP_MAPPER_ADDRESS`) | `terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2` |
| cLUNC (`VITE_LUNC_C_TOKEN_ADDRESS`) | `terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg` |
| cUSTC (`VITE_USTC_C_TOKEN_ADDRESS`) | `terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch` |
| Router (soft launch, unchanged) | `terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw` |

Wrap-mapper `Config { fee_bps }` on mainnet: **100** (1%).

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

1. Query `fee_bps` via `queryWrapMapperConfig` (LCD) — do not hardcode mainnet 100 in app logic without a fallback query path.
2. Direct wrap/unwrap quotes use `netAfterWrapMapperFee`; native-input swaps net CW20 after tax **and** mapper fee where applicable (`netCw20AfterNativeWrap`).
3. Unwrap / native-output simulation must net `fee_bps` on the unwrap leg (aligns with router `minimum_receive` on post-unwrap net — **R3**).
4. Burn tax on native transfers is **additional** to mapper `fee_bps` — [`AGENTS_NATIVE_WRAP_TAX.md`](./AGENTS_NATIVE_WRAP_TAX.md).
5. Enabling wrap in Coolify does **not** require redeploying factory/router; it requires correct `VITE_*` + image rebuild.

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

- Native burn tax: [`AGENTS_NATIVE_WRAP_TAX.md`](./AGENTS_NATIVE_WRAP_TAX.md)
- Router `minimum_receive` / unwrap net: [`AGENTS_ROUTER_MINIMUM_RECEIVE.md`](./AGENTS_ROUTER_MINIMUM_RECEIVE.md)
- Swap pause / rate-limit CTAs: [`AGENTS_FRONTEND_SWAP_SAFETY_CTA.md`](./AGENTS_FRONTEND_SWAP_SAFETY_CTA.md)
- Soft launch (pre-wrap): [`AGENTS_MAINNET_SOFT_LAUNCH.md`](./AGENTS_MAINNET_SOFT_LAUNCH.md)
- Architecture: [`NATIVE_TOKEN_WRAPPING.md`](../NATIVE_TOKEN_WRAPPING.md)
- Runbook: [`docs/runbooks/mainnet-soft-launch.md`](../docs/runbooks/mainnet-soft-launch.md)
