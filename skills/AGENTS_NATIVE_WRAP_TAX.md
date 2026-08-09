# Agent playbook: Native wrap burn tax (#342)

Use when changing **native LUNC/USTC → CW20** multi-msg flows (`wrap_deposit` + CW20 action) or debugging **`Overflow: Cannot Sub`** on wrapped amounts.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [NATIVE_TOKEN_WRAPPING.md § Tax-aware amounts](../NATIVE_TOKEN_WRAPPING.md) | Invariant: gross native deposit ≠ CW20 minted balance |
| [`nativeTransferTax.ts`](../frontend-dapp/src/utils/nativeTransferTax.ts) | `netUlunaAfterTransferTax`, LCD tax query (60s cache) |
| [`router.ts`](../frontend-dapp/src/services/terraclassic/router.ts) | `executeNativeSwap` / `simulateNativeSwap` use net CW20 offer after wrap; `netCw20AfterNativeWrap` also nets wrap-mapper `fee_bps` (#507) |
| [`skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md`](./AGENTS_MAINNET_WRAP_ENABLEMENT.md) | Post-SL5 mainnet wrap env + cLUNC/cUSTC UX (#507) |
| [`PoolPage.tsx`](../frontend-dapp/src/pages/PoolPage.tsx) | Native-wrap provide: allowances + `provide_liquidity` use net raw amounts |
| [GitLab **#342**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/342) | `Cannot Sub with 9950000 and 10000000` repro |

## Rules of thumb

1. **`wrap_deposit` coins** stay **gross** (user-entered native amount).
2. **CW20 `send` / LP asset amounts** after wrap use **net** post-tax minted units.
3. **Simulation** for native-input swaps must use the same net offer as execute.
4. Do not hardcode `0.995` — query LCD tax rate (LocalTerra often ~`0.0005`).
5. **Pool provide auto-fill** ([#480](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/480)): ratio math uses net post-tax amounts; when the counterpart side is native-wrap, invert net → gross via `grossUlunaForTargetNet` in [`nativeTransferTax.ts`](../frontend-dapp/src/utils/nativeTransferTax.ts). See [`AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md`](./AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md).
6. After burn tax, wrap-mapper **`fee_bps`** also skims minted/unwrap amounts ([#507](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/507)). Use `netCw20AfterNativeWrap` in [`router.ts`](../frontend-dapp/src/services/terraclassic/router.ts); playbook [`AGENTS_MAINNET_WRAP_ENABLEMENT.md`](./AGENTS_MAINNET_WRAP_ENABLEMENT.md).

## Related

- Gas / wrap envelope: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)
- E2E: `e2e/wrap-swap.spec.ts`, `e2e/wrap-pool.spec.ts` (`e2e-tx`, 1 worker)
