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

## Canonical docs

- Invariant **L8:** [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md)
- Product/integrator: [`docs/limit-orders.md`](../docs/limit-orders.md), [`docs/integrators.md`](../docs/integrators.md)
- ADR: [`docs/adr/0001-hybrid-quoting-and-routing.md`](../docs/adr/0001-hybrid-quoting-and-routing.md)

## Tests to run after changes

```bash
cd smartcontracts && cargo test -p cl8y-dex-tests limit_order_tests::
cd frontend-dapp && npm test -- src/services/terraclassic/__tests__/pair.test.ts
cd indexer && cargo test api_route_solve
cd frontend-dapp && pnpm exec playwright test e2e/hybrid-swap.spec.ts   # strict E2E — see AGENTS_E2E_HYBRID_SWAP.md (#193)
```

## Related frontend skills

- [`AGENTS_E2E_HYBRID_SWAP.md`](./AGENTS_E2E_HYBRID_SWAP.md) — Playwright hybrid tx path without conditional skips ([#193](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193))
- [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) — route spread preflight uses per-hop `hybrid_simulation` only.
