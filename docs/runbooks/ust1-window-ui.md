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

## Published mainnet addresses (Phase 2)

| Role | Address |
|------|---------|
| vFDUSD | `terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3` |
| UST1 | `terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72` |
| ust1-oracle | `terra1fmht0t6svq3n24zx03nkfja0m40zhfyyxkdcvlrkl6u7gfe6aagq4gch8n` (code **11568**) |
| ust1-window | `terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2` (code **11566**) |

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

## Code map

| Path | Role |
|------|------|
| [`Ust1Page.tsx`](../../frontend-dapp/src/pages/Ust1Page.tsx) | Deposit / Withdraw UI |
| [`ust1Window.ts`](../../frontend-dapp/src/services/terraclassic/ust1Window.ts) | LCD `effective_swap` + CW20 Send |
| [`ust1WindowMath.ts`](../../frontend-dapp/src/utils/ust1WindowMath.ts) | INV-SWAP quote math |
| [`ust1WindowGates.ts`](../../frontend-dapp/src/utils/ust1WindowGates.ts) | Pause / stale / limit CTA gates |
| [`navItems.ts`](../../frontend-dapp/src/components/common/navItems.ts) | `UST1_NAV_ITEM` + `includeUst1` |
| [`tokenRegistry.ts`](../../frontend-dapp/src/utils/tokenRegistry.ts) + `tokenlist/` | UST1 / vFDUSD metadata + logos |

## Verification

```bash
make verify-issue-506
# or:
bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
  src/utils/__tests__/ust1WindowMath.test.ts \
  src/utils/__tests__/ust1WindowGates.test.ts \
  src/services/terraclassic/__tests__/ust1Window.test.ts \
  src/pages/Ust1Page.test.tsx \
  src/components/common/navItems.test.ts \
  src/services/terraclassic/__tests__/terraGas.retailShapes.test.ts
```

### Mainnet smoke (post-deploy)

1. Open `https://dex.cl8y.com/ust1` — nav label **UST1** (not Mint).
2. Side-by-side: LCD `effective_swap` vs UI receive for a probe amount.
3. Small deposit vFDUSD→UST1 and withdraw UST1→vFDUSD; record tx hashes on [#506](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506) / [#502](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/502).

## Related

- Soft-launch faucet (must stay separate): [`soft-launch-faucet.md`](./soft-launch-faucet.md), [`AGENTS_SOFT_LAUNCH_FAUCET.md`](../../skills/AGENTS_SOFT_LAUNCH_FAUCET.md)
- Shell nav: [`AGENTS_FRONTEND_SHELL_NAV.md`](../../skills/AGENTS_FRONTEND_SHELL_NAV.md)
- Agent playbook: [`AGENTS_UST1_WINDOW_UI.md`](../../skills/AGENTS_UST1_WINDOW_UI.md)
- Gas inventory: [`AGENTS_TERRACLASSIC_GAS.md`](../../skills/AGENTS_TERRACLASSIC_GAS.md)
- Routes table: [`docs/frontend.md`](../frontend.md)
