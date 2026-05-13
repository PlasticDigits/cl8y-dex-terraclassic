# Agent playbook: frontend parked-expired limit refunds (GitLab #141)

Use when wiring or QA’ing **maker recovery** after `limit_order_expired_parked`: indexer **`lifecycle_status: parked_expired`**, pair **`ClaimExpiredLimitOrder`**, dApp **Claim refund**.

## Preconditions

- Indexer exposes **`GET /api/v1/pairs/{addr}/limit-placements`** with **`lifecycle_status`** (default feed = **`active` + `parked_expired`**) — [GitLab **#142**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/142), [`docs/indexer-invariants.md`](../docs/indexer-invariants.md). **#141:** indexer ingests **`limit_order_expired_parked`** even when LCD wasm logs flatten multiple actions and **`swap` is last** in the attribute stream ([`parser.rs`](../indexer/src/indexer/parser.rs)).
- Pair wasm emits **`limit_order_expired_parked`** / **`claim_expired_limit_order`** — [GitLab **#120**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120), [`docs/limit-orders.md` § Expiry](../docs/limit-orders.md#expiry-expires_at).

## Canonical code / docs

| Area | Location |
|------|----------|
| Shared UI | [`frontend-dapp/src/components/trade/LimitOrderMyPlacementsPanel.tsx`](../frontend-dapp/src/components/trade/LimitOrderMyPlacementsPanel.tsx) (`/limits` + `/trade` ticket) |
| Lifecycle helpers | [`frontend-dapp/src/utils/limitPlacementLifecycle.ts`](../frontend-dapp/src/utils/limitPlacementLifecycle.ts) |
| Claim execute | [`frontend-dapp/src/services/terraclassic/pair.ts`](../frontend-dapp/src/services/terraclassic/pair.ts) `claimExpiredLimitOrder` |
| Gas | [`frontend-dapp/src/services/terraclassic/transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts) `CLAIM_EXPIRED_LIMIT_ORDER_GAS_LIMIT` |
| Revert copy | [`frontend-dapp/src/utils/limitClaimUserMessage.ts`](../frontend-dapp/src/utils/limitClaimUserMessage.ts) → [`humanizeTerraTxError.ts`](../frontend-dapp/src/utils/humanizeTerraTxError.ts) |
| Indexer client | [`frontend-dapp/src/services/indexer/client.ts`](../frontend-dapp/src/services/indexer/client.ts) optional **`?status=`** on limit-placements |
| Product invariants | [`docs/limit-orders.md` § dApp retail form — #141](../docs/limit-orders.md#dapp-retail-form-wires-invariants) |

## Rules of thumb

1. **Paused pairs (L6 / GitLab [#120](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/120)):** **`ClaimExpiredLimitOrder`** is **blocked on-chain** while **`is_paused`**. The dApp disables **Claim refund** under pause — **do not** present a claim button that would burn gas on a guaranteed `Paused` revert; mirror the cancel button’s pause awareness.
2. **Escrow decimals:** **`remaining_escrow`** is shown in **bid → token1**, **ask → token0** units (same as cancel/refund paths).
3. **Invalidations:** after a successful claim tx, invalidate **`limitPlacements`**, limit book previews, and **`tokenBalance`** so balances and rows refresh.
4. **Automation / bots:** poll **`parked_expired`** (indexer or LCD **`ExpiredLimitRefund`**) and confirm the pair is **not** paused before broadcasting claim — see [`AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md) cross-links.
