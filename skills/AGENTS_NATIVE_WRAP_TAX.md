# Agent playbook: Native wrap amounts (#342 / #512)

Use when changing **native LUNC/USTC → CW20** multi-msg flows (`wrap_deposit` + CW20 action) or debugging **`Overflow: Cannot Sub`** on wrapped amounts.

> **#512 correction:** Classic does **not** burn-tax `MsgExecuteContract` funds. CW20 minted after wrap is **mapper `fee_bps` only**. Earlier #342 guidance that applied Terraswap-style tax to wrap mint **over-deducted** quotes (e.g. display 9751 vs receive 9800 at 200 bps). Burn tax applies on **unwrap InstantWithdraw** (`BankMsg::Send`) — see [`AGENTS_WRAP_UNWRAP_BURN_TAX.md`](./AGENTS_WRAP_UNWRAP_BURN_TAX.md).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [NATIVE_TOKEN_WRAPPING.md § Tax-aware amounts](../NATIVE_TOKEN_WRAPPING.md) | Wrap mint vs unwrap InstantWithdraw tax |
| [`nativeTransferTax.ts`](../frontend-dapp/src/utils/nativeTransferTax.ts) | Classic `burn_tax_rate` multiply tax (unwrap / BankMsg::Send) |
| [`router.ts`](../frontend-dapp/src/services/terraclassic/router.ts) | `netCw20AfterNativeWrap` = fee only; `netNativeAfterUnwrap` = fee then tax |
| [`skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md`](./AGENTS_WRAP_UNWRAP_BURN_TAX.md) | #512 unwrap incidence + W8–W11 |
| [`skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md`](./AGENTS_MAINNET_WRAP_ENABLEMENT.md) | Post-SL5 mainnet wrap env + cLUNC/cUSTC UX (#507) |
| [`PoolPage.tsx`](../frontend-dapp/src/pages/PoolPage.tsx) | Native-wrap provide: allowances + `provide_liquidity` use post-fee mint |
| [GitLab **#342**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/342) | Historical `Cannot Sub` — often `fee_bps` skim, not burn tax |
| [GitLab **#512**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/512) | Unwrap tax stacks; wrap display fix |

## Rules of thumb

1. **`wrap_deposit` coins** stay **gross** (user-entered native amount).
2. **CW20 `send` / LP asset amounts** after wrap use **post–mapper-fee** minted units (`netCw20AfterNativeWrap`) — **not** burn-taxed.
3. **Simulation** for native-input swaps must use the same post-fee offer as execute.
4. **Unwrap / native-output** quotes must net InstantWithdraw burn tax after mapper fee (**W9**).
5. Query LCD `burn_tax_rate` via `/terra/tax/v1beta1/params` — do not hardcode forever; default fallback is columbus-5 **0.015**.
6. **Pool provide auto-fill**: ratio math uses post-fee amounts; invert via `amountForTargetNetAfterWrapMapperFee` (no tax gross-up on wrap). See [`AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md`](./AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md).

## Related

- Unwrap burn tax / exchange trap: [`AGENTS_WRAP_UNWRAP_BURN_TAX.md`](./AGENTS_WRAP_UNWRAP_BURN_TAX.md)
- Gas / wrap envelope: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)
- E2E: `e2e/wrap-swap.spec.ts`, `e2e/wrap-pool.spec.ts` (`e2e-tx`, 1 worker)
