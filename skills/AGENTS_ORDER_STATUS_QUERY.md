# Agent playbook: pair `OrderStatus` query (GitLab #505)

Use when integrating **on-chain contract consumers** (custody vaults, grid bots, multi-contract strategies) that hold local limit `order_id`s and need **typed absence**, not an untyped CosmWasm `StdError` from `LimitOrder`.

## Canonical API

| Item | Location |
|------|----------|
| Types | [`dex_common::pair`](../smartcontracts/packages/dex-common/src/pair.rs) — `OrderStatus`, `OrderStatusResponse`, `QueryMsg::OrderStatus` |
| Handler | [`orderbook::query_order_status`](../smartcontracts/contracts/pair/src/orderbook.rs) |
| Dispatch | [`pair::query`](../smartcontracts/contracts/pair/src/contract.rs) |
| Re-exports | [`pair::msg`](../smartcontracts/contracts/pair/src/msg.rs) |
| Tests | [`order_status_tests.rs`](../smartcontracts/tests/src/order_status_tests.rs); unit: `orderbook::order_status_tests` |
| Invariant | **L21** in [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) |
| Product docs | [`docs/limit-orders.md`](../docs/limit-orders.md), [`docs/integrators.md`](../docs/integrators.md) |

## Classification (existing storage only)

| Check | Status |
|-------|--------|
| `ORDERS.may_load(order_id)` → `Some` | `Active` (+ owner/side/price/remaining/expires_at) |
| else `EXPIRED_LIMIT_CLAIMS.may_load` → `Some` | `ParkedRefund` (+ owner/side/remaining/expires_at; **price is `None`**) |
| else | `Unknown` (all metadata `None`) |

- `order_id == 0` → **error** (not `Unknown`).
- Dual presence in both maps is an invariant violation (parks unlink first); handler still prefers `Active` if constructible in tests.

## Hard rules for consumers

1. **Only a successful decode of `OrderStatusResponse` may yield `Unknown`.** LCD / `ContractQuery` / JSON failures stay `Err` — never map them to `Unknown`.
2. **`Unknown` ≠ fully filled.** It also covers cancel, claim-after-park, never-placed, and pre-history ids. Unlocking vault settlement solely on `Unknown` is unsafe; keep a local cancel/place ledger or use off-chain indexing for fill-vs-cancel.
3. **Do not weaken legacy queries.** `LimitOrder` still errors when missing; `ExpiredLimitRefund` still returns `None`. Prefer `OrderStatus` for new contract callers.
4. **Read-only / no state growth.** No tombstones, no `OWNER_ORDERS`, no execute dual-writes, no `Protocol {}` capability menu in this feature.
5. **Pause:** query remains available while the pair is paused (read-only); cancel/claim stay gated (**L6**).

## Related playbooks

- Parked refund UX / claim paths: [`AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](./AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md)
- Localnet automation that watches parked rows: [`AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md)

## Out of scope (do not sneak in)

- Indexer HTTP proxy for `OrderStatus` (optional follow-up)
- Terminal subtypes (`FullyExecuted` / `Cancelled` / `NotFound`)
- Owner inventory pagination / backfill
