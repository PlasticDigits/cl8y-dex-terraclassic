# Post-swap hooks (burn, tax, LP burn)

These contracts implement `HookMsg::AfterSwap` callbacks registered on pairs via factory governance.

## `AfterSwap` fields on hybrid swaps (invariant L7)

When the pair executes a **hybrid** swap (constant-product pool **and** limit book in one tx), the hook receives **`commission_amount` and `spread_amount` for the pool leg only**. Total user output in `return_asset.amount` includes **book net + pool net**. Limit-book taker fees are attributed on `limit_order_fill` events, not in the hook’s `commission_amount`.

Integrators must not treat `commission_amount` as the full protocol fee for the transaction. See **[`docs/integrators.md`](../../../docs/integrators.md)** (§ Hybrid swaps and post-swap hooks).
