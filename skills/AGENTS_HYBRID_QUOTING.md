# Agent skill: Hybrid quoting (L8 / GitLab #190)

## When to use

You are changing **swap quotes**, **router simulation**, **indexer route solve**, or **frontend preflight** for CL8Y DEX on Terra Classic.

## Rules (post-#190)

1. **Never** call pair `simulation` or `reverse_simulation` LCD/CosmWasm queries — they were removed.
2. **Always** use `hybrid_simulation` / `hybrid_reverse_simulation`.
3. **Pool-only** (reserves only, no book walk):
   - **Rust:** `dex_common::pair::pool_only_hybrid_params(offer_amount)` (forward), `pool_only_hybrid_template()` (reverse ratio template).
   - **TypeScript:** `poolOnlyHybridParams(offerAmount)` / `poolOnlyHybridTemplate()` in [`frontend-dapp/src/services/terraclassic/poolOnlyHybrid.ts`](../frontend-dapp/src/services/terraclassic/poolOnlyHybrid.ts).
4. **Router** `simulate_swap_operations`: `terra_swap.hybrid: null` still means pool-only on-chain — router maps that to `pool_only_hybrid_params` per hop.
5. **Book-inclusive quotes:** non-zero `book_input`; must match execute `HybridSwapParams` for the same snapshot (queries do not park expired orders).
6. **CL8Y fee discounts (GitLab #238):** pass optional `trader` on pair `HybridSimulation` / `HybridReverseSimulation` (and router `SimulateSwapOperations`) so discounted wallets get execute-matching output; omit for full-fee quotes. Optional `sender` when `trader` differs from the CW20 sender (trusted router). Sim is read-only (no deregister). Rust helpers: `hybrid_simulation_with_trader`, `hybrid_simulation_undiscounted`.

## Canonical docs

- Invariant **L8:** [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md)
- Invariant **L9** (`max_spread` pool + hybrid): [`dex_common::max_spread`](../smartcontracts/packages/dex-common/src/max_spread.rs), [GitLab **#197**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/197), [`docs/integrators.md`](../docs/integrators.md#slippage-max_spread-and-belief_price-hybrid)
- Product/integrator: [`docs/limit-orders.md`](../docs/limit-orders.md), [`docs/integrators.md`](../docs/integrators.md)
- ADR: [`docs/adr/0001-hybrid-quoting-and-routing.md`](../docs/adr/0001-hybrid-quoting-and-routing.md)
- Fee discount tiers + sim `trader`: [`skills/AGENTS_FEE_DISCOUNT_TIERS.md`](./AGENTS_FEE_DISCOUNT_TIERS.md) · [GitLab **#238**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/238)
- Frontend preflight math: [`docs/swap-max-spread-ux.md`](../docs/swap-max-spread-ux.md)

## Tests to run after changes

```bash
cd smartcontracts && cargo test -p dex-common max_spread
cd smartcontracts && cargo test -p cl8y-dex-tests limit_order_tests::hybrid_simulation_matches_execute_with_fee_discount
cd smartcontracts && cargo test -p cl8y-dex-tests limit_order_tests::hybrid_max_spread
cd smartcontracts && cargo test -p cl8y-dex-tests test_swap_max_spread
cd frontend-dapp && npm test -- src/services/terraclassic/__tests__/pair.test.ts
cd indexer && cargo test api_route_solve
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/hybrid-swap.spec.ts --project=e2e-tx   # strict E2E — see AGENTS_E2E_HYBRID_SWAP.md (#193)
```

## Related frontend skills

- [`AGENTS_E2E_HYBRID_SWAP.md`](./AGENTS_E2E_HYBRID_SWAP.md) — Playwright hybrid tx path without conditional skips ([#193](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193))
- [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) — route spread preflight uses per-hop `hybrid_simulation` only.
