# Agent skill: Limit order place/cancel Playwright E2E (GitLab #195)

## When to use

You are changing **limit order on-chain browser tests**, **E2E LCD pair selection**, or **LocalTerra dev-wallet provisioning** that must keep the default CI/local Playwright path from silently skipping when the first factory pair is paused or prerequisites are missing.

## Invariants (strict default)

| Prerequisite | Enforced by | Failure mode |
|--------------|-------------|--------------|
| LCD up + `.env.local` | `e2e/global-setup.ts` | Setup throws before tests |
| Dev wallet CW20 balances (bid escrow = token1) | `scripts/e2e-provision-dev-wallet.sh` | **Insufficient Balance** on Place → test fails |
| Unpaused dual-CW20 pair exists | `requireLimitTxPair()` + LCD `is_paused` in `e2e/helpers/lcd.ts` | Hard fail (not `test.skip`) |
| UI pause banner absent after selection | `skipOrFailIfPairPaused()` from `hybrid-e2e.ts` | Hard fail (L6) |
| Place / cancel wasm actions | `limit-orders-tx.spec.ts` LCD poll | Expect `place_limit_order`, `cancel_limit_order` |
| 5-rung ladder (one tx) | `limit-orders-tx.spec.ts` ladder test | Expect `place_limit_order_batch` + `place_limit_order` |

Set **`PLAYWRIGHT_SKIP_CHAIN=1`** (or legacy `REQUIRE_LOCALTERRA=0`) only for UI-only local runs; helpers fall back to documented `test.skip`. Default CI must not set it ([#201](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201), [`AGENTS_E2E_STRICT_CHAIN.md`](./AGENTS_E2E_STRICT_CHAIN.md)).

## Files

| Path | Role |
|------|------|
| [`frontend-dapp/e2e/limit-orders-tx.spec.ts`](../frontend-dapp/e2e/limit-orders-tx.spec.ts) | Funded place + cancel txs |
| [`frontend-dapp/e2e/limit-orders.spec.ts`](../frontend-dapp/e2e/limit-orders.spec.ts) | UI smoke (no chain) |
| [`frontend-dapp/e2e/helpers/limit-e2e.ts`](../frontend-dapp/e2e/helpers/limit-e2e.ts) | Pair pick + CTA guards |
| [`frontend-dapp/e2e/helpers/lcd.ts`](../frontend-dapp/e2e/helpers/lcd.ts) | `queryPairPaused`, `firstUnpausedDualCwPair` |
| [`scripts/e2e-provision-dev-wallet.sh`](../scripts/e2e-provision-dev-wallet.sh) | CW20 mint floor for dev wallet |
| [`frontend-dapp/e2e/README.md`](../frontend-dapp/e2e/README.md) | Operator runbook |

## Run after changes

```bash
docker compose up -d localterra
bash scripts/deploy-dex-local.sh
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/limit-orders-tx.spec.ts --project=e2e-tx
```

## Cross-links

- Product / pause semantics: [`docs/limit-orders.md`](../docs/limit-orders.md) (L6), [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) **L6**
- Testing matrix: [`docs/testing.md`](../docs/testing.md) § E2E Tests
- Hybrid swap E2E (shared pause helper): [`AGENTS_E2E_HYBRID_SWAP.md`](./AGENTS_E2E_HYBRID_SWAP.md) ([#193](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193))
- Dev wallet funding: [`AGENTS_BUNDLE_DEV_WALLET.md`](./AGENTS_BUNDLE_DEV_WALLET.md)
- GitLab [#195](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/195)
- Batch / ladder limits [#206](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/206): [`AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](./AGENTS_LIMIT_ORDER_BATCH_LADDER.md)
- Strict chain umbrella: [#201](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201), [`AGENTS_E2E_STRICT_CHAIN.md`](./AGENTS_E2E_STRICT_CHAIN.md)
