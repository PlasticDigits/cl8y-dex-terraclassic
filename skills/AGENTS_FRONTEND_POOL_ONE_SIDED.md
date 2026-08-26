# Agent playbook: one-sided pool add / withdraw (auto zap + wrap)

Use when changing **retail `/pool` one-sided liquidity**: pick one wallet token + pair to add, or one wallet LP + token to withdraw as. Wrap/unwrap is implied by the token. Spec, AC, path tests, and attack plan live on [GitLab **#533**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/533).

`#531` how-to (**H531-3**) describes Manage → four actions. Two-sided **Provide Liquidity** is the empty-pool first deposit. Zap lives on the same toolbar. `#489` (no always-on essays), `#366` (IL), `#147` / `#213` (gas + native wrap), `#462` (pre-sign), `#497` (5% slippage) still apply. Pair-scoped IA: [`AGENTS_FRONTEND_POOL_MANAGE_IA.md`](./AGENTS_FRONTEND_POOL_MANAGE_IA.md) (`#660`).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#533**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/533) | Spec: AC1–AC13, T/U/P paths, A1–A20 |
| [`PoolPage.tsx`](../frontend-dapp/src/pages/PoolPage.tsx) | Search + how-to + table; zap lives under pair Manage |
| [`pair.ts`](../frontend-dapp/src/services/terraclassic/pair.ts) | `provideLiquidity` / `withdrawLiquidity` |
| [`router.ts`](../frontend-dapp/src/services/terraclassic/router.ts) | Wrap-in + swap + `unwrap_output` |
| [`poolProvideCounterpart.ts`](../frontend-dapp/src/utils/poolProvideCounterpart.ts) | Two-sided auto-fill — **not** a zap solver |
| [`usePortfolioLpBalances.ts`](../frontend-dapp/src/hooks/usePortfolioLpBalances.ts) | Wallet LP scan for withdraw picker |
| [`poolLpHowtoCopy.ts`](../frontend-dapp/src/utils/poolLpHowtoCopy.ts) | #531 copy (update H531-3) |
| [`docs/frontend.md`](../docs/frontend.md) § Pool provide / how-to | Engineering invariants |
| [`docs/user-lunc-liquidity.md`](../docs/user-lunc-liquidity.md) | Retail backup |

## Invariants (Z533)

1. **Z533-1** — Zap add controls: token (wallet holdings `> 0`), amount. Pair is the expanded Manage row (no second pair picker). No wrap checkbox, no second amount, no on-card slippage chips.
2. **Z533-2** — Zap withdraw controls: this pair’s LP, token to receive, amount. No “receive wrapped” checkbox. No other-pair LP picker.
3. **Z533-3** — Native `uluna` / `uusd` wrap or unwrap automatically. Pools still hold CW20 only. Do not send native into the pair or router.
4. **Z533-4** — Zap-in targets the **post-swap** pool ratio. Retail must not `provide_liquidity` off-ratio (no silent donate).
5. **Z533-5** — Empty pool: zap disabled. First deposit stays two-sided **Provide Liquidity** (`MINIMUM_LIQUIDITY`). Copy: `Empty pool. Use Provide Liquidity.`
6. **Z533-6** — Factory pairs only. Pool-only zap swap (`poolOnlyHybridParams`). Same quote snapshot on submit as Swap (#356).
7. **Z533-7** — Slippage on every leg (`min_return` / `slippage_tolerance` / `min_assets`). Stop passing `slippage_tolerance: null`. Default 5% from Settings (#497).
8. **Z533-8** — Unwrap **only** the zap-out amount. Never unwrap the rest of the wallet’s cLUNC/cUSTC (today’s withdraw loop is the bug).
9. **Z533-9** — Gas/Max cover the full wrap + swap + provide (or withdraw + swap + unwrap) sequence. Keep pause, blacklist, mapper pause, treasury match, IL, clickwrap, NFA, Expert Mode, pre-sign.
10. **Z533-10** — LP amounts use CW20 decimals **18**, not the UI `LP_DECIMALS = 6` leftover. No APR / farm chrome. No new pair/router `Zap` execute unless LocalTerra rehearsal proves multi-msg cannot work.

## Rules of thumb

1. Frontend orchestration on existing messages: optional `wrap_deposit` → pair/router swap → allowances → `provide_liquidity`. Withdraw: LP send → swap other side → optional unwrap of **that** amount.
2. Off-pair input: `GET /route/solve` into a pair leg then zap, or one-sentence “No route”.
3. Two-sided provide/withdraw is a peer Manage action (empty-pool bootstrap and exact-ratio deposit). Do **not** hide it under Advanced.
4. Do **not** change pair mint/burn math, fee treasury, or wrap-mapper fee bps in this issue.
5. Zap fee math must use `useFeeDiscountRegistryStatus(pairAddr)` (**I14** / [#537](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/537)). An unwired pair charges full `fee_bps` on-chain — do not apply a wallet `get_discount` to the split.
6. Prefer one `executeTerraContractMulti` when gas fits; rollback both allowances in one multi-msg on provide failure (#147).

## Verify

```bash
make verify-issue-533
make verify-issue-559
make test-frontend
# scoped:
#   oneSidedLiquidity + oneSidedLiquidityTx + oneSidedLiquidityQuote + PoolPage + poolLpHowtoCopy
# Playwright smoke (5 workers, PLAYWRIGHT_SKIP_CHAIN=1):
#   frontend-dapp/e2e/pool-one-sided-533.spec.ts
# LocalTerra tx (1 worker; wrap-mapper split-fee instantiate is #539):
#   frontend-dapp/e2e/pool-one-sided-533-tx.spec.ts
#   make verify-issue-539
```

Issue: [GitLab **#533**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/533) (AC1–AC13, T1–T10, U1–U12, P1–P10, A1–A20).

## Related

- Zap execution floors (quote vs execute): [`AGENTS_FRONTEND_POOL_ZAP_FLOORS.md`](./AGENTS_FRONTEND_POOL_ZAP_FLOORS.md) (`#559` — **Z559-1–Z559-4**; `make verify-issue-559`)
- Pair Manage IA: [`AGENTS_FRONTEND_POOL_MANAGE_IA.md`](./AGENTS_FRONTEND_POOL_MANAGE_IA.md) (`#660` — **M660-1–M660-8**; `make verify-issue-660`)
- Retail how-to: [`AGENTS_FRONTEND_POOL_LP_HOWTO.md`](./AGENTS_FRONTEND_POOL_LP_HOWTO.md) (`#531` — rewrite H531-3)
- Two-sided auto-fill / withdraw preview: [`AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md`](./AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md) (`#480`)
- Advanced provide labels + wrap default: [`AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md`](./AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md) (`#661` — **P661-1–P661-12**; do not add a wrap checkbox here)
- Pre-sign: [`AGENTS_FRONTEND_POOL_SIGNING_CONFIRMATION.md`](./AGENTS_FRONTEND_POOL_SIGNING_CONFIRMATION.md) (`#462`)
- Copy: [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) (`#489`)
- Slippage default: [`AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md`](./AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md) (`#497`)
- Wrap fees / burn tax: [`AGENTS_WRAP_UNWRAP_BURN_TAX.md`](./AGENTS_WRAP_UNWRAP_BURN_TAX.md) (`#512`)
- Wrap enablement: [`AGENTS_MAINNET_WRAP_ENABLEMENT.md`](./AGENTS_MAINNET_WRAP_ENABLEMENT.md) (`#507`)
