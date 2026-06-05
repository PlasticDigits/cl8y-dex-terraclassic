# Agent skill: unified `max_spread` (pool + hybrid, GitLab #197)

## When to use

You are changing **slippage checks**, **`max_spread` / `belief_price`**, hybrid swap execute paths, or frontend **price impact / submit gating** for CL8Y DEX on Terra Classic.

## Product rule (invariant L9)

One shared implementation: [`dex_common::max_spread`](../smartcontracts/packages/dex-common/src/max_spread.rs). Pool-only swaps are `book_net_return = 0`.

| Mode | Pass condition |
|------|----------------|
| **No `belief_price`** | `(min(pool_spread, pool_gross) + book_shortfall) / (pool_gross + book_net) ≤ max_spread` (strict `>` fails). `book_shortfall` vs pool net rate when both legs present ([#273](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/273)). Hybrid with `book_input > 0` and `pool_input > 0` also requires `pool_input ≥ 10%` of offer and `pool_net > 0` ([#307](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/307)). Pure-book (`pool_input = 0`) unchanged — needs `belief_price` or `min_receive`. |
| **`belief_price` set** | `(expected − actual) / expected ≤ max_spread` where `expected = offer / belief_price` and `actual = book_net + pool_net + pool_commission` |

Default `max_spread` when omitted: **1%**.

## Do not

- Reintroduce pool-only-only spread math on hybrid executes — the denominator must include **book net to taker**.
- Assume `AfterSwap.spread_amount` includes book slippage — it remains **pool leg only** (L7); L9 is the execute guard, not hook attrs.

## Canonical docs

- Invariant **L9:** [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md)
- Integrators: [`docs/integrators.md`](../docs/integrators.md#slippage-max_spread-and-belief_price-hybrid)
- ADR: [`docs/adr/0001-hybrid-quoting-and-routing.md`](../docs/adr/0001-hybrid-quoting-and-routing.md)
- dApp UX / preflight: [`docs/swap-max-spread-ux.md`](../docs/swap-max-spread-ux.md)
- Hybrid quoting (L8): [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md)

## Tests to run after changes

```bash
cd smartcontracts && cargo test -p dex-common max_spread
cd smartcontracts && cargo test -p cl8y-dex-tests hybrid_max_spread
cd smartcontracts && cargo test -p cl8y-dex-tests test_swap_max_spread
cd frontend-dapp && npm test -- src/utils/__tests__/swapMaxSpread.test.ts
```

Integration tests live in [`smartcontracts/tests/src/limit_order_tests.rs`](../smartcontracts/tests/src/limit_order_tests.rs):

- `hybrid_max_spread_exact_tolerance_succeeds`
- `hybrid_max_spread_tighter_than_simulation_rejected`
- `hybrid_belief_price_max_spread_rejects_shortfall_on_total_output`
- `hybrid_no_belief_book_far_below_pool_rejected` ([#273](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/273))
- `hybrid_no_belief_dust_pool_leg_rejected` ([#307](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/307))

Pool-only regression: `test_swap_max_spread` in [`smartcontracts/tests/src/lib.rs`](../smartcontracts/tests/src/lib.rs).

## Frontend mirror

TypeScript preflight (no `belief_price`): [`frontend-dapp/src/utils/swapMaxSpread.ts`](../frontend-dapp/src/utils/swapMaxSpread.ts) — keep in sync with `dex_common::max_spread`.
