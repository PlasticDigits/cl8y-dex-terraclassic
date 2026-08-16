# Agent playbook: My open limits Cancel (stale ● row)

Use when changing **Cancel** on **My open limits**, book row `×`, **Cancel all mine**, or Advanced cancel-by-id so a maker can cancel a resting order — or see why they cannot ([GitLab **#530**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/530)).

Issue **#530** — open-row reconciliation is implemented. Report class: `●order #1 · Sell UST1 · 82.044… · placed 2026-08-15T14:21:43` (indexer-`active` ask after the id left `ORDERS`, usually a full hybrid fill). `#135` cancel-vs-indexer, `#141`/`#142` park lifecycle, `#505` `OrderStatus` / **L21**, and **L6** pause still apply. Do **not** treat a green `●` row with a disabled or hidden **Cancel** as done.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#530**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/530) | Full spec: current code, guardrails, AC, path tests, attack plan |
| [`docs/limit-orders.md` § Open-row Cancel](../docs/limit-orders.md#open-row-cancel-reconciliation-gitlab-530) | Invariants **F530-1–F530-8** |
| [`docs/frontend.md` § My open limits Cancel](../docs/frontend.md#open-limits-cancel-reconciliation) | dApp wiring |
| [`limitPlacementOpenReconcile.ts`](../frontend-dapp/src/utils/limitPlacementOpenReconcile.ts) | LCD + indexer classification |
| [`useLimitOrderStatuses.ts`](../frontend-dapp/src/hooks/useLimitOrderStatuses.ts) | LCD `OrderStatus` + local cancel ids |
| [`useTraderLimitFills.ts`](../frontend-dapp/src/hooks/useTraderLimitFills.ts) | Fill evidence for `Unknown` |
| [`LimitOrderMyPlacementsPanel.tsx`](../frontend-dapp/src/components/trade/LimitOrderMyPlacementsPanel.tsx) | Active row + Cancel / Claim / gone labels |
| [`useLimitOrderCancelMutation.ts`](../frontend-dapp/src/hooks/useLimitOrderCancelMutation.ts) | Shared cancel + LCD/indexed preflight |
| [`AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](./AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md) | Parked rows use **Claim**, not Cancel |
| [`AGENTS_ORDER_STATUS_QUERY.md`](./AGENTS_ORDER_STATUS_QUERY.md) | LCD `Active` / `ParkedRefund` / `Unknown` — `Unknown` ≠ fill |
| [`indexer/src/indexer/parser.rs`](../indexer/src/indexer/parser.rs) | `process_limit_order_fill` does **not** flip placement lifecycle |

## Target behavior

Reconcile “open” with chain before offering Cancel:

| LCD `OrderStatus` | UI |
|-------------------|----|
| `Active` | **Cancel** enabled (unless pause / blacklist / gas) |
| `ParkedRefund` | **Claim refund** / **Claim dust** — not Cancel |
| `Unknown` | No fake `●` Cancel. Classify via indexer fills vs cancellations: **Filled** / **Already cancelled** / **No longer on the book** |
| Query failed | Do **not** map to `Unknown` (**L21**). Keep Cancel on indexer-active rows. |

Prefer LCD `OrderStatus` (or equivalent) plus indexer placement / fill / cancel. Do not treat `Unknown` as proof of fill (**L21**).

## Rules of thumb

1. **Triage first** — record LCD status, indexer `lifecycle_status`, cancellation row, fills, `is_paused`, wallet vs `owner` before changing contracts.
2. **Owner-only / L6** — never cancel another wallet’s id; pause still blocks cancel/claim on-chain and in the UI.
3. **No double refund** — Cancel only while `ORDERS` has the row. After park, Claim only.
4. **Disabled Cancel must say why** — paused, restricted, already gone, filled, or claim instead. Do not leave a mute **Cancel**.
5. **`/trade` reachability** — compact panel is **above** `trade-ticket-submit-footer`. Cancel must be clickable (`elementFromPoint` hits the button, not `trade-limit-submit`).
6. **Invert is display-only** — **Sell UST1** (#524) still cancels factory `order_id` on the selected pair. Place-price bounds (**#529** / **L20**) do not apply to cancel; see [`AGENTS_LIMIT_PRICE_DECIMALS.md`](./AGENTS_LIMIT_PRICE_DECIMALS.md).
7. **Copy (#489)** — no `token0` / `token1` / raw `bid`/`ask`. One-sentence blocking errors.
8. **Testids** — keep `trade-cancel-placement-{id}`, `limits-page-cancel-placement-{id}`, `trade-book-cancel-{bid\|ask}-{id}`, `trade-ticket-placements-anchor`.

## Verify

Issue: [GitLab **#530**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/530) (AC1–AC8, I1–I10, Playwright P1–P7, attack A1–A10).

```bash
make verify-issue-530
make test-frontend
# scoped: LimitOrderMyPlacementsPanel + limitPlacementOpenReconcile + useLimitOrderCancelMutation
# chain: make has-localterra || make setup-cloud-localterra
# Playwright: e2e/limit-orders-tx.spec.ts + /trade compact cancel
```

## Related

- Parked claim: [`AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](./AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md) (`#141`)
- `OrderStatus`: [`AGENTS_ORDER_STATUS_QUERY.md`](./AGENTS_ORDER_STATUS_QUERY.md) (`#505`)
- Ticket CTA dock (Cancel under Place limit): [`AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md`](./AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md) (`#527`)
- UST1 invert: [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) (`#524`)
