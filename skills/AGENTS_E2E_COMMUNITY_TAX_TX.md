# Agent playbook: community-tax Playwright e2e-tx (GitLab #622)

Use when changing **tax-pair Swap / Pool / Limits / Trade Market** on-chain Playwright, `community-tax-env.ts` smoke-vs-tx pins, or LocalTerra seed funding that the tax spec consumes.

This is **retail path coverage** for the #620 QA tax/EMBER market. Do **not** add pair/router FoT math (**H-01** / **T592-1**). Do **not** turn hybrid off (**#596**). Wrap stays cLUNC/cUSTC. Gem `e2e-tx` specs stay on the first dual-CW20 pair (EMBER/CORAL).

Parent seed: [`AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md`](./AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md) (**L620**). Strict chain: [`AGENTS_E2E_STRICT_CHAIN.md`](./AGENTS_E2E_STRICT_CHAIN.md). Extra-debit Max: [`AGENTS_FRONTEND_CREATE_TOKEN.md`](./AGENTS_FRONTEND_CREATE_TOKEN.md) (**C593-9**). You Receive net: [`AGENTS_INDEXER_TAX_AWARE_ROUTING.md`](./AGENTS_INDEXER_TAX_AWARE_ROUTING.md) (**R615**). Hybrid always-on: [`AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md`](./AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md) (**H596**).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#622**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/622) | Tax-pair e2e-tx matrix |
| [`community-tax-tx.spec.ts`](../frontend-dapp/e2e/community-tax-tx.spec.ts) | P0 sell/buy/provide/limit + P1 Trade Market quote |
| [`community-tax-e2e.ts`](../frontend-dapp/e2e/helpers/community-tax-e2e.ts) | LCD TaxPreview / balances / Max |
| [`community-tax-env.ts`](../frontend-dapp/e2e/helpers/community-tax-env.ts) | Smoke columbus-5 bake vs tx `.env.local` |
| [`communityTaxTxEnv.ts`](../frontend-dapp/src/utils/communityTaxTxEnv.ts) | Pin parse + columbus-5 reject |
| [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) **E622-1** | Invariant table |

## Invariants **E622-1–E622-8**

1. **E622-1 — e2e-tx only, 1 worker.** `community-tax-tx.spec.ts` matches `*-tx.spec.ts`. Never add tax txs to `e2e-smoke` as silent skips. Shared `test1` mnemonic — do not raise `e2e-tx` workers. Gem helpers (`firstDualCwPair`, hybrid/wrap seed) prefer **EMBER/CORAL** from `.env.local` and skip the pinned tax market so leftover tax pairs cannot steal the default pair.
2. **E622-2 — fail closed on missing seed.** Missing `VITE_TOKEN_COMMUNITY_TAX_ADDRESS` / `VITE_PAIR_COMMUNITY_TAX_EMBER` / local launcher after a default seed deploy **fails** the spec. No `test.skip` on the default path. `PLAYWRIGHT_SKIP_CHAIN=1` does not run this file (smoke-only).
3. **E622-3 — local pins, not columbus-5.** Tx path reads `frontend-dapp/.env.local`. Code ids **11611 / 11612 / 11613 / 11614 / 11619 / 11620 / 11621 / 11622 / 8654** and launcher `terra126pr5…ahzwze` are rejected. Smoke `/token/create` may still bake columbus-5 when skip-tax deploy is used.
4. **E622-4 — sell extra-debit.** Max is `maxDeclaredForExtraDebitSell` (not 100% wallet). After a small sell, user debit == `TaxPreview.debit`; pair credit == Send `amount` (inbound 1:1). CTA not blocked. Do not submit Max against the seed LP (wallet is ~1e18 raw). Thin tax/EMBER vs hub fair-rate may trip the 30% expected-slippage gate — enable **Expert Mode** (do not turn hybrid off).
5. **E622-5 — buy You Receive is net.** Display matches LCD user credit after outbound split (**R615**). User credit + sink == pair debit. Do not assert raw pair `Simulation`.
6. **E622-6 — provide / place are 1:1.** Pool `TransferFrom` pair delta == declared. Limit **Send** is honest (`tax_kind=honest`, no sell extra-debit). Pair takes **maker fee** from escrow and pays it to the maker in the same tx — wallet/pair deltas are `declared - maker_fee_amount`, not `declared`. Extra-debit (`userDebit > declared`) on provide or place is a product bug. Cancel pair debit equals remaining escrow (no sell extra-debit). Pair→EOA refund is buy-classified: user + treasury = remaining. QA seed has ExemptionDirectory off, so wallet restore is **net** (`buy_bps`) — do not assert `userAfterCancel === userBefore`. Directory skip (#609) is 1:1. After leftover retries, cancel by LCD `order_id` (Advanced form) — indexer `last-placed-order-id` is maxId and can point at a cancelled row. Withdraw UI on the tax pair is not P0 (Advanced `<details>` toggle is flaky; wrap-pool covers withdraw). Limit price helper must parse **`Ref N`** as well as `Current: N`. Last-placed id is **`Order #N`** — parse case-insensitively. Place gas is batch `n=1` at **1.18M** (`PLACE_LIMIT_ORDER_BATCH_BASE_GAS_LIMIT` **1M** + 180k) so tax `Send` does not OOG at 580k.
7. **E622-7 — hybrid stays on.** Specs must not set `pool_only` or restore a hybrid checkbox to go green. Router hops tax the original trader (**T592-13**); copy is option-2 (`Buy/sell tax applies on every listed-pair swap`).
8. **E622-8 — not a FoT license / not wrap.** Do not put tax pairs into `multihop-hybrid-tx` CORAL→IRON unless the route is explicit. Do not force wrap-swap/pool onto the tax template. Invoice Create/Enable (P2) needs the local UST1 stand-in — do not Send 50 columbus-5 UST1.

`e2e-provision-dev-wallet.sh` must not Mint tax tokens and must not `Transfer` `test1→test1` (no-op that fails extra-debit on leftovers). Pinned token below the floor is **fail_seed**. See [`cw20-funding-kind.sh`](../scripts/lib/cw20-funding-kind.sh) `classify_tax_provision_action` and **L620-2**.

## P1 / P2 remaining

| Item | Status |
|------|--------|
| P1 Trade Market `GET /route/solve` net quote + option-2 copy | **Done** (quote only; no second market submit) |
| P1 official-router ≥2hop with tax as offer or hop | **Not in this MR** — needs an explicit tax hop; do not reuse CORAL→IRON |
| P1 hybrid fill + tax accounting on tax/EMBER | **Not in this MR** — seed book is the first gem pair |
| P1 one-sided zap in/out floors (#559) | **Not in this MR** |
| P2 on-chain Create Token free + paid SKU | **Not in this MR** — local UST1 stand-in |
| P2 Enable Feature via launcher hook | **Not in this MR** (**T606-7**) |
| P2 ExemptionDirectory Max fail-closed (#609) | **Not in this MR** — QA seed has no directory SKU |

## Verify

```bash
make verify-issue-622
# Live (seed deploy + indexer). VERIFY_ISSUE_622_CHAIN=1 starts the indexer and
# merges PLAYWRIGHT_WEB_PORT (default 3173) into CORS_ORIGINS (#625 leftover #2):
# VERIFY_ISSUE_622_CHAIN=1 make verify-issue-622
# Manual:
# PLAYWRIGHT_WEB_PORT=3173 bash scripts/e2e-start-indexer.sh
# CI=1 PLAYWRIGHT_WEB_PORT=3173 PLAYWRIGHT_BASE_URL=http://127.0.0.1:3173 \
#   bash scripts/with-node.sh --cwd frontend-dapp -- \
#   ./node_modules/.bin/playwright test e2e/community-tax-tx.spec.ts --project=e2e-tx
```

`make verify-issue-596` / `501` / `533` stay the gem/hybrid/zap gates — this spec does not replace them. Leftover live after !417 (P0 LCD extra-debit on a clean seed, treasury ≠ test1): [`AGENTS_POST_MERGE_OPS_625.md`](./AGENTS_POST_MERGE_OPS_625.md) ([#625](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/625)).

## Do not

- `test.skip` when the tax pair is missing.
- Bake columbus-5 **11611** into the tx path.
- Assert 1:1 user debit on sell or buy.
- Treat limit place as a sell (extra-debit here is a bug).
- Turn hybrid off to get a green sell.
- Send Enable Feature invoices to the **token**.
