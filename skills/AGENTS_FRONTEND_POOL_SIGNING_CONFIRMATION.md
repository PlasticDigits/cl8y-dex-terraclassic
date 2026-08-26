# Agent playbook: pool signing confirmation (SEC-I05)

Use when changing **pre-sign pool provide/withdraw summary** copy, labeled confirmation fields, or tests that guard against wallet phishing / signing confusion ([#462](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/462), SEC-I05 F-03).

## Product invariant

Before the wallet extension opens, pool provide and withdraw surfaces must show a **compact labeled pre-sign summary** with:

| Field | Label | `data-testid` suffix |
|-------|-------|----------------------|
| Action | `Provide Liquidity` or `Withdraw Liquidity` | `-action` |
| Pair | `{tokenA} / {tokenB}` symbols | `-pair` |
| Amount | provide: `{amountA} {symA} + {amountB} {symB}` using the **selected** input tickers (native when wrap is on — [#661](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/661)); withdraw: `{lpAmount} LP` plus `~{expectedA} {symA} + ~{expectedB} {symB}` when pool data is available | `-amount` |
| Chain | active network full label (`LocalTerra`, `Terra Classic`, …) | `-chain` |

Panel roots: **`pool-provide-pre-submit-summary`** and **`pool-withdraw-pre-submit-summary`** on `/pool`.

Pool forms already expose token inputs above the submit button; the card repeats **security anchors only** (no swap-style intro paragraph) to limit cognitive overload while preserving SEC-D11 parity.

## Code map

| Concern | Location |
|--------|----------|
| Shared component | [`PoolPreSubmitSummary.tsx`](../frontend-dapp/src/components/pool/PoolPreSubmitSummary.tsx) |
| **Provide / withdraw** wiring | [`PoolPage.tsx`](../frontend-dapp/src/pages/PoolPage.tsx) — above submit when amounts are entered |
| Chain label | [`networkDisplay.ts`](../frontend-dapp/src/utils/networkDisplay.ts) — `getNetworkBadgeCopy().fullLabel` |
| Unit tests | [`PoolPreSubmitSummary.test.tsx`](../frontend-dapp/src/components/pool/__tests__/PoolPreSubmitSummary.test.tsx) |
| Integration tests | [`PoolPage.test.tsx`](../frontend-dapp/src/pages/PoolPage.test.tsx) |

## Verification commands

```bash
make test-frontend
cd frontend-dapp && npm run test:run -- \
  src/components/pool/__tests__/PoolPreSubmitSummary.test.tsx \
  src/pages/PoolPage.test.tsx
```

## Rules of thumb

1. **Do not** remove individual field `data-testid`s — SEC-I05 regression tests assert each labeled row.
2. **Do not** drop the pair row — amounts alone do not identify the pool contract.
3. Keep the card **compact** (four rows: action, pair, amount, chain); avoid duplicating the swap intro copy.
4. Swaps use [`SwapPreSubmitSummary`](../frontend-dapp/src/components/swap/SwapPreSubmitSummary.tsx) — see [`AGENTS_FRONTEND_SWAP_SIGNING_CONFIRMATION.md`](./AGENTS_FRONTEND_SWAP_SIGNING_CONFIRMATION.md).
5. Provide auto-fill / withdraw preview math: [`AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md`](./AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md) ([#480](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/480)).

## Related

- Swap pre-sign: [`AGENTS_FRONTEND_SWAP_SIGNING_CONFIRMATION.md`](./AGENTS_FRONTEND_SWAP_SIGNING_CONFIRMATION.md)
- Limit pre-sign: [`LimitOrderPreSubmitSummary.tsx`](../frontend-dapp/src/components/trade/LimitOrderPreSubmitSummary.tsx) ([#461](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/461))
- Docs: [`docs/frontend.md` § Pool pre-sign summary](../docs/frontend.md#pool-page-pre-sign-summary)
- One-sided Add/Withdraw (#533): [`AGENTS_FRONTEND_POOL_ONE_SIDED.md`](./AGENTS_FRONTEND_POOL_ONE_SIDED.md) — `pool-one-sided-add-pre-submit` / `pool-one-sided-withdraw-pre-submit`
- Advanced provide labels + wrap default (#661): [`AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md`](./AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md)
