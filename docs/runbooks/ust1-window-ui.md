# UST1 oracle window UI (`/ust1`)

Operator + agent runbook for the always-on **vFDUSD ↔ UST1** mint/redeem surface on `dex.cl8y.com` ([GitLab **#506**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506), parent [#502](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502)).

Upstream contracts: [ust1-window](https://gitlab.com/PlasticDigits/ust1-window) (Phase 2 deploy [#19](https://gitlab.com/PlasticDigits/ust1-window/-/issues/19)).

## Invariants

| ID | Rule |
|----|------|
| **U1** | Frontend exposes UST1 nav + execute path only when `VITE_UST1_WINDOW_ADDRESS`, `VITE_UST1_TOKEN_ADDRESS`, and `VITE_VFDUSD_TOKEN_ADDRESS` are all set (`isUst1WindowEnabled`). |
| **U2** | Route is **`/ust1`** with nav label **UST1** — never reuse **`/mint`** or label **Mint** (faucet is [#473](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/473)). |
| **U3** | Deposit/withdraw execute only as CW20 `Send` to **ust1-window** — never AMM router / pair swap. |
| **U4** | Quotes and CTA gates use on-chain `effective_swap` (rate, `fee_bps`, pause, oracle pause/age, per-tx + rolling 24h). Client integer math mirrors `ust1-common` INV-SWAP-001/002; never invent rates. |
| **U5** | Allowlist: deposit pay token = **vFDUSD** only; withdraw pay token = **UST1** only. |
| **U6** | Soft-launch faucet mintables must not include UST1 or vFDUSD. |
| **U7** | Coolify / prod `VITE_*` must be columbus-5 addresses below — never LocalTerra defaults on `dex.cl8y.com`. |
| **U8** | CW20 `send` → `{ deposit }` / `{ withdraw }` maps to **`UST1_WINDOW_SEND_GAS_LIMIT` (800k)** in `getGasLimitForTx` (retail inventory **G-RETAIL-1**). |
| **U9** | Swap / Trade unfunded UST1 pay Guides to **`/ust1?direction=deposit&amount=`** (human vFDUSD from inverse `effective_swap`, clamped to per-tx / rolling remaining). Typed size above remaining capacity must **not** promise a failing mint. `/ust1` ignores hostile query strings and never auto-submits. Identity is `VITE_UST1_TOKEN_ADDRESS`, not the ticker. [#678](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/678) **A678**. |

## Published mainnet addresses (Phase 2)

| Role | Address |
|------|---------|
| vFDUSD | `terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3` |
| UST1 | `terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72` |
| ust1-oracle | `terra1fmht0t6svq3n24zx03nkfja0m40zhfyyxkdcvlrkl6u7gfe6aagq4gch8n` (code **11568**) |
| ust1-window | `terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2` (code **11618**, was **11566**) |

Approved window params: `fee_bps=100`, per-tx **1000** UST1, rolling 24h **10000** UST1.

## Coolify / env

Bake into the frontend image (`docker/frontend/Dockerfile` ARGs + Coolify build-args):

```bash
VITE_UST1_WINDOW_ADDRESS=terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2
VITE_UST1_TOKEN_ADDRESS=terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72
VITE_VFDUSD_TOKEN_ADDRESS=terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3
VITE_UST1_ORACLE_ADDRESS=terra1fmht0t6svq3n24zx03nkfja0m40zhfyyxkdcvlrkl6u7gfe6aagq4gch8n
```

Also listed in [`frontend-dapp/.env.example`](../../frontend-dapp/.env.example). Rebuild Coolify after changing build-args (Vite bakes env at build time).

Indexer protocol fees ([#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614) / **PFee-13**) need the **indexer** pin (Vite does not feed ingest):

```bash
UST1_WINDOW_ADDRESS=terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2
```

See [`coolify.env.example`](../../deployments/mainnet-ust1-wrap/coolify.env.example), [`AGENTS_INDEXER_UST1_WINDOW_FEES.md`](../../skills/AGENTS_INDEXER_UST1_WINDOW_FEES.md) (**I614**), and [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](../../skills/AGENTS_FRONTEND_PROTOCOL_STATS.md).

## Code map

| Path | Role |
|------|------|
| [`Ust1Page.tsx`](../../frontend-dapp/src/pages/Ust1Page.tsx) | Deposit / Withdraw UI |
| [`ust1Window.ts`](../../frontend-dapp/src/services/terraclassic/ust1Window.ts) | LCD `effective_swap` + CW20 Send |
| [`ust1WindowMath.ts`](../../frontend-dapp/src/utils/ust1WindowMath.ts) | INV-SWAP quote math + inverse deposit (`vfdusdInForTargetUst1`, #678) |
| [`ust1WindowGates.ts`](../../frontend-dapp/src/utils/ust1WindowGates.ts) | Pause / stale / limit CTA gates |
| [`ust1AcquirePrefill.ts`](../../frontend-dapp/src/utils/ust1AcquirePrefill.ts) | Swap Guide query parse + clamp |
| [`swapPayAcquireGuidance.ts`](../../frontend-dapp/src/utils/swapPayAcquireGuidance.ts) | Swap/Trade shortfall helper |
| [`navItems.ts`](../../frontend-dapp/src/components/common/navItems.ts) | `UST1_NAV_ITEM` + `includeUst1` |
| [`tokenRegistry.ts`](../../frontend-dapp/src/utils/tokenRegistry.ts) + `tokenlist/` | UST1 / vFDUSD metadata + logos |

## Verification

```bash
make verify-issue-506
make verify-issue-678
# or:
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
  src/utils/__tests__/ust1WindowMath.test.ts \
  src/utils/__tests__/ust1WindowGates.test.ts \
  src/services/terraclassic/__tests__/ust1Window.test.ts \
  src/pages/Ust1Page.test.tsx \
  src/components/common/navItems.test.ts \
  src/services/terraclassic/__tests__/terraGas.retailShapes.test.ts

# Playwright (mocked LCD effective_swap; no LocalTerra ust1-window required):
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/ust1-window.spec.ts --project=e2e-smoke
```

### Mainnet smoke (post-deploy)

1. Open `https://dex.cl8y.com/ust1` — nav label **UST1** (not Mint).
2. Side-by-side: LCD `effective_swap` vs UI receive for a probe amount.
3. Small deposit vFDUSD→UST1 and withdraw UST1→vFDUSD; record tx hashes on [#506](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506) / [#502](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502).

## Related

- Soft-launch faucet (must stay separate): [`soft-launch-faucet.md`](./soft-launch-faucet.md), [`AGENTS_SOFT_LAUNCH_FAUCET.md`](../../skills/AGENTS_SOFT_LAUNCH_FAUCET.md)
- Shell nav: [`AGENTS_FRONTEND_SHELL_NAV.md`](../../skills/AGENTS_FRONTEND_SHELL_NAV.md)
- Agent playbook: [`AGENTS_UST1_WINDOW_UI.md`](../../skills/AGENTS_UST1_WINDOW_UI.md)
- Swap / Trade acquire Guide: [`AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md`](../../skills/AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md) ([#678](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/678))
- Phase 5 ops (oracle age, pause, inventory): [`ust1-wrap-production-ops.md`](./ust1-wrap-production-ops.md), [`AGENTS_UST1_WRAP_PRODUCTION_OPS.md`](../../skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md) ([#503](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503))
- Gas inventory: [`AGENTS_TERRACLASSIC_GAS.md`](../../skills/AGENTS_TERRACLASSIC_GAS.md)
- Routes table: [`docs/frontend.md`](../frontend.md)
- Registry: [`deployments/mainnet-ust1-wrap/REGISTRY.md`](../../deployments/mainnet-ust1-wrap/REGISTRY.md)
- Protocol treasury fees: [`AGENTS_INDEXER_UST1_WINDOW_FEES.md`](../../skills/AGENTS_INDEXER_UST1_WINDOW_FEES.md) (**I614**, [#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614))
