# Agent playbook: frontend parked-expired limit refunds (GitLab #141)

Use when wiring or QA'ing **maker recovery** after `limit_order_expired_parked`: indexer **`lifecycle_status: parked_expired`**, pair **`ClaimExpiredLimitOrder`** / **`ClaimExpiredLimitOrders`**, dApp **Claim refund** and **Claim all parked**.

## Preconditions

- Indexer exposes **`GET /api/v1/pairs/{addr}/limit-placements`** with **`lifecycle_status`** (default feed = **`active` + `parked_expired`**) — [GitLab **#142**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/142), [`docs/indexer-invariants.md`](../docs/indexer-invariants.md). **#141:** indexer ingests **`limit_order_expired_parked`** even when LCD wasm logs flatten multiple actions and **`swap` is last** in the attribute stream ([`parser.rs`](../indexer/src/indexer/parser.rs)).
- Pair wasm emits **`limit_order_expired_parked`** / **`claim_expired_limit_order`** — [GitLab **#120**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120), [**#250**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/250) (≤ **15** parks per hybrid swap book walk; skipped expired head stays on book until maker cancel or later park), [**#254**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/254) (**`MAX_SCAN_STEPS`** bounds total book walk iterations — **500**, decoupled from maker cap), [**#262**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/262) (**`MAX_MAKER_FILLS_HARD_CAP` = 100**, dApp gas ceiling **15M**), [**#264**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/264) (**match-time dust flush** — post-fill `0 < remaining < 10` auto-parks with **`force_expired=true`**; does **not** consume the 15 time-expiry park cap), [`docs/limit-orders.md` § Expiry](../docs/limit-orders.md#expiry-expires_at), [§ Match-time dust flush](../docs/limit-orders.md#match-time-dust-flush-gitlab-264).
- Batch claim execute + gas landed in [GitLab **#246**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/246). **Claim all parked** UI: [GitLab **#253**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/253). **Confirm gas + Playwright E2E:** [GitLab **#259**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/259).

## Canonical code / docs

| Area | Location |
|------|----------|
| Shared UI | [`frontend-dapp/src/components/trade/LimitOrderMyPlacementsPanel.tsx`](../frontend-dapp/src/components/trade/LimitOrderMyPlacementsPanel.tsx) (`/limits` + `/trade` ticket) |
| Claim mutation hook | [`frontend-dapp/src/hooks/useLimitExpiredClaimMutation.ts`](../frontend-dapp/src/hooks/useLimitExpiredClaimMutation.ts) (single id → `claimExpiredLimitOrder`; batch → `claimExpiredLimitOrders`) |
| Batch helpers | [`frontend-dapp/src/utils/limitExpiredClaimBatch.ts`](../frontend-dapp/src/utils/limitExpiredClaimBatch.ts) (normalize, chunk ≤ **100**, confirm copy + **est. LUNC gas** — #259; cap raised #263) |
| Lifecycle helpers | [`frontend-dapp/src/utils/limitPlacementLifecycle.ts`](../frontend-dapp/src/utils/limitPlacementLifecycle.ts) |
| Claim execute | [`frontend-dapp/src/services/terraclassic/pair.ts`](../frontend-dapp/src/services/terraclassic/pair.ts) `claimExpiredLimitOrder`, `claimExpiredLimitOrders` |
| Gas | [`frontend-dapp/src/services/terraclassic/terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts) `gasLimitForLimitOrderCancelBatch`; single `CLAIM_EXPIRED_LIMIT_ORDER_GAS_LIMIT` in [`transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts) |
| Revert copy | [`frontend-dapp/src/utils/limitClaimUserMessage.ts`](../frontend-dapp/src/utils/limitClaimUserMessage.ts) → [`humanizeTerraTxError.ts`](../frontend-dapp/src/utils/humanizeTerraTxError.ts) |
| Indexer client | [`frontend-dapp/src/services/indexer/client.ts`](../frontend-dapp/src/services/indexer/client.ts) optional **`?status=`** on limit-placements |
| Product invariants | [`docs/limit-orders.md` § dApp retail form — #141 / #253](../docs/limit-orders.md#dapp-retail-form-wires-invariants) |

## Rules of thumb

1. **Paused pairs (L6 / GitLab [#120](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120)):** **`ClaimExpiredLimitOrder`** and **`ClaimExpiredLimitOrders`** are **blocked on-chain** while **`is_paused`**. The dApp disables **Claim refund** and **Claim all parked** under pause — **do not** present claim controls that would burn gas on a guaranteed `Paused` revert; mirror the cancel button's pause awareness.
2. **Escrow decimals:** **`remaining_escrow`** is shown in **bid → token1**, **ask → token0** units (same as cancel/refund paths).
3. **Claim all parked (#253):** show only when **≥ 2** indexed **`parked_expired`** rows for the connected wallet on the current pair. Submit **deduped** `order_id`s from those rows only. Cap **100** ids per tx (`MAX_LIMIT_BATCH_RUNGS_HARD_CAP`, raised [#263](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/263)); chunk larger sets with **one confirm per chunk** (each confirm includes **est. LUNC gas** from `gasLimitForLimitOrderCancelBatch` — #259). Single parked row: per-row **Claim refund** only. **Off-chain clean:** permissionless on-chain **`clean_limit_book`** parks rows for makers — no dApp button in #263; see [limit-orders.md § Permissionless limit book clean](../docs/limit-orders.md#permissionless-limit-book-clean).
4. **Invalidations:** after a successful claim tx (single or batch), invalidate **`limitPlacements`**, limit book previews, and **`tokenBalance`** so balances and rows refresh.
5. **Automation / bots:** poll **`parked_expired`** (indexer or LCD **`ExpiredLimitRefund`**) and confirm the pair is **not** paused before broadcasting claim — prefer batch when many rows; see [`AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md) cross-links.

## QA checklist (#253, #259)

- [ ] **Claim all parked (N)** visible on `/limits` and `/trade` compact panel when **N ≥ 2** parked rows; hidden for **N = 1**.
- [ ] Batch tx claims all listed ids (≤ 30 per tx); chunked confirm when **N > 30**; each confirm shows **est. LUNC gas** (#259).
- [ ] **Claim all** + per-row buttons disabled while pair paused (`Unavailable (pair paused)`).
- [ ] Successful claim removes parked rows after indexer catch-up; balances refresh.
- [ ] Vitest: [`useLimitExpiredClaimMutation.test.ts`](../frontend-dapp/src/hooks/__tests__/useLimitExpiredClaimMutation.test.ts), [`LimitOrderMyPlacementsPanel.test.tsx`](../frontend-dapp/src/components/trade/__tests__/LimitOrderMyPlacementsPanel.test.tsx), [`limitExpiredClaimBatch.test.ts`](../frontend-dapp/src/utils/__tests__/limitExpiredClaimBatch.test.ts).
- [ ] Playwright **`e2e-tx`:** [`limit-orders-claim-all-tx.spec.ts`](../frontend-dapp/e2e/limit-orders-claim-all-tx.spec.ts) — harness + LCD `claim_expired_limit_orders_batch` ([#259](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/259)).
