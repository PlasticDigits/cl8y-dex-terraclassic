# Agent skill: Hybrid swap Playwright E2E (GitLab #193)

## When to use

You are changing **hybrid swap browser tests**, **E2E global setup**, or **LocalTerra seed scripts** that must keep the default CI/local Playwright path from silently skipping when chain prerequisites are missing.

## Invariants (strict default)

| Prerequisite | Enforced by | Failure mode |
|--------------|-------------|--------------|
| LCD up + `.env.local` | `e2e/global-setup.ts` | Setup throws before tests |
| Dev wallet CW20 balances | `scripts/e2e-provision-dev-wallet.sh` | Hybrid swap CTA shows **Insufficient Balance** → test fails |
| Dual-CW20 factory pair | `requireDualCwPair()` in `e2e/helpers/hybrid-e2e.ts` | Hard fail (not `test.skip`) |
| Resting bid on first dual pair | `scripts/e2e-seed-hybrid-book.sh` | **No Route** / empty book → fail; **re-run safe** when bid head already set ([GitLab **#138**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)) |
| Pair not paused | `skipOrFailIfPairPaused()` | Hard fail (L6) |
| Hybrid tx wasm attrs | `hybrid-swap.spec.ts` on-chain case | Expect `limit_order_fill` + `book_return_amount` > 0 on `swap` |

Set **`PLAYWRIGHT_SKIP_CHAIN=1`** (or legacy `REQUIRE_LOCALTERRA=0`) only for UI-only local runs; helpers fall back to documented `test.skip`. Default CI path must not set it ([**#201**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201), [`AGENTS_E2E_STRICT_CHAIN.md`](./AGENTS_E2E_STRICT_CHAIN.md)).

## Files

| Path | Role |
|------|------|
| [`frontend-dapp/e2e/hybrid-swap.spec.ts`](../frontend-dapp/e2e/hybrid-swap.spec.ts) | UI disclosure + funded hybrid tx |
| [`frontend-dapp/e2e/fee-discount-quote-245.spec.ts`](../frontend-dapp/e2e/fee-discount-quote-245.spec.ts) | **#245** Trade market quote=execute + Swap `trader` on route/solve + router sim |
| [`frontend-dapp/e2e/helpers/hybrid-e2e.ts`](../frontend-dapp/e2e/helpers/hybrid-e2e.ts) | Strict prerequisite helpers |
| [`scripts/e2e-seed-hybrid-book.sh`](../scripts/e2e-seed-hybrid-book.sh) | Idempotent bid seed (`E2E_HYBRID_SEED_*`); **`order_book_head`** LCD payload is a bare **`u64`**, not `{ head_order_id }` — use `order_book_head_id_from_payload` ([GitLab **#138**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)) |
| [`scripts/e2e-provision-dev-wallet.sh`](../scripts/e2e-provision-dev-wallet.sh) | CW20 mint floor for dev wallet |
| [`frontend-dapp/e2e/README.md`](../frontend-dapp/e2e/README.md) | Operator runbook |

## Run after changes

```bash
docker compose up -d localterra
bash scripts/deploy-dex-local.sh
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/hybrid-swap.spec.ts --project=e2e-tx
```

## Cross-links

- Product / wasm semantics: [`docs/limit-orders.md`](../docs/limit-orders.md) (hybrid attrs, L8)
- Testing matrix: [`docs/testing.md`](../docs/testing.md) § E2E Tests
- Hybrid quoting (L8, not E2E-specific): [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md)
- Dev wallet funding: [`AGENTS_BUNDLE_DEV_WALLET.md`](./AGENTS_BUNDLE_DEV_WALLET.md)
- Limit place/cancel tx E2E (unpaused pair pick): [`AGENTS_E2E_LIMIT_ORDERS_TX.md`](./AGENTS_E2E_LIMIT_ORDERS_TX.md) ([#195](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/195))
- GitLab [#193](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193)
- Strict chain umbrella: [#201](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/201), [`AGENTS_E2E_STRICT_CHAIN.md`](./AGENTS_E2E_STRICT_CHAIN.md)
- **#138 verification:** cosmes patch integrity test + hybrid seed idempotency — [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md), [`AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](./AGENTS_FRONTEND_RISK_DISCLAIMERS.md)
