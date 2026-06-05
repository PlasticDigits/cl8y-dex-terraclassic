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
5. **Book-inclusive quotes:** non-zero `book_input`; must match execute `HybridSwapParams` for the same snapshot (queries do not park expired orders). **`book_start_hint`** must be same-side as the simulated matcher leg (**L17**, [GitLab **#272**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/272)) — see [`AGENTS_BOOK_MATCH_HINT_SECURITY.md`](./AGENTS_BOOK_MATCH_HINT_SECURITY.md).
6. **Reverse sim gas ([GitLab #257](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/257)):** `HybridReverseSimulation` seeds search from pool constant-product math ([`hybrid_reverse.rs`](../smartcontracts/contracts/pair/src/hybrid_reverse.rs)); ≤ **32** full hybrid sim calls per LCD query. Minimal-offer invariant unchanged — regression `hybrid_reverse_sim_minimal_offer_invariant`.
7. **CL8Y fee discounts (GitLab #238, off-chain wiring #245, on-chain cache #251, route cache tier #283):** pass optional `trader` on pair `HybridSimulation` / `HybridReverseSimulation` (and router `SimulateSwapOperations`) so discounted wallets get execute-matching output; omit for full-fee quotes. Optional `sender` when `trader` differs from the CW20 sender (trusted router). **Frontend:** thread connected wallet through `simulateSwap`, `preflightSwapRouteSpread`, and indexer `getRouteSolve` / `postRouteSolve`. **Indexer:** optional `trader` / `sender` on `GET/POST /api/v1/route/solve`; hybrid GET cache keys include normalized `trader` when set ([#245](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245)) and the resolved **`discount_tier`** (`traders.tier_id` for `trader` if set, else `sender`; unknown → 0) so same-tier callers share the cache ([#283](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/283)). Sim is read-only (no deregister, no cache write); within **300s** of a prior execute it **reads** the pair’s on-pair discount cache ([`DISCOUNT_CACHE_TTL_SECONDS`](../smartcontracts/packages/dex-common/src/pair.rs), [fee-discount-tiers § I9](../docs/reference/fee-discount-tiers.md)). Rust helpers: `hybrid_simulation_with_trader`, `hybrid_simulation_undiscounted`.

## Canonical docs

- Invariant **L8:** [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) (reverse search bounds: [GitLab **#257**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/257))
- Invariant **L17** (`book_start_hint` side validation): [GitLab **#272**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/272), [`AGENTS_BOOK_MATCH_HINT_SECURITY.md`](./AGENTS_BOOK_MATCH_HINT_SECURITY.md)
- Invariant **L9** (`max_spread` pool + hybrid): [`dex_common::max_spread`](../smartcontracts/packages/dex-common/src/max_spread.rs), [GitLab **#197**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/197), [`docs/integrators.md`](../docs/integrators.md#slippage-max_spread-and-belief_price-hybrid)
- Invariant **L10** (hybrid execute CW20 aggregation / gas): [GitLab **#248**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/248), [`docs/limit-orders.md` § Execution order](../docs/limit-orders.md#execution-order-in-execute_swap), [`skills/AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) — sim path unchanged; only execute submessage count differs.
- **Frontend fee envelopes ([GitLab #249](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/249), book walk [#260](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/260)):** `Fee.gas` follows `max_maker_fills` on the submit payload **plus** conservative scan/park overhead when `book_input > 0` via [`hybridSwapGas.ts`](../frontend-dapp/src/services/terraclassic/hybridSwapGas.ts); single-hop routes skip the router ([`swapRouting.ts`](../frontend-dapp/src/services/terraclassic/swapRouting.ts)). Does not change pair execute semantics.
- Product/integrator: [`docs/limit-orders.md`](../docs/limit-orders.md), [`docs/integrators.md`](../docs/integrators.md)
- ADR: [`docs/adr/0001-hybrid-quoting-and-routing.md`](../docs/adr/0001-hybrid-quoting-and-routing.md)
- Fee discount tiers + sim `trader`: [`skills/AGENTS_FEE_DISCOUNT_TIERS.md`](./AGENTS_FEE_DISCOUNT_TIERS.md) · [GitLab **#238**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/238) · off-chain wiring [GitLab **#245**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/245) · route cache tier [GitLab **#283**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/283) · QA: `make verify-issue-245` ([`AGENTS_QA_DEPLOY_VERIFY.md`](./AGENTS_QA_DEPLOY_VERIFY.md))
- Frontend preflight math: [`docs/swap-max-spread-ux.md`](../docs/swap-max-spread-ux.md)

## Tests to run after changes

```bash
cd indexer && cargo test hybrid_cache_key --lib   # #283 discount bps in hybrid GET cache key; #324 mmf bucketing
cd indexer && cargo test concurrent_solve --lib   # #324 bounded concurrent candidate eval
make verify-issue-324   # #324 lib + route_solve_get_cache integration (needs make setup-indexer-postgres on Cloud Agent)
cd indexer && cargo test route_solve_get_cache_tier -j 1 -- --test-threads=1   # #306 HTTP tier cache isolation
cd smartcontracts && cargo test -p cl8y-dex-pair aggregation_tests
cd smartcontracts && cargo test -p cl8y-dex-tests limit_order_tests::hybrid_swap_two_makers
cd smartcontracts && cargo test -p dex-common max_spread
cd smartcontracts && cargo test -p cl8y-dex-tests limit_order_tests::hybrid_simulation_matches_execute_with_fee_discount
cd smartcontracts && cargo test -p cl8y-dex-tests limit_order_tests::hybrid_reverse_sim_minimal_offer_invariant
cd smartcontracts && cargo test -p cl8y-dex-pair hybrid_reverse
cd smartcontracts && cargo test -p cl8y-dex-pair book_start_hint_side_tests
cd smartcontracts && cargo test -p cl8y-dex-tests hybrid_wrong_side_book_start_hint
cd smartcontracts && cargo test -p cl8y-dex-tests limit_order_tests::hybrid_max_spread
cd smartcontracts && cargo test -p cl8y-dex-tests test_swap_max_spread
cd frontend-dapp && npm test -- src/services/terraclassic/__tests__/pair.test.ts
cd indexer && cargo test api_route_solve
bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/hybrid-swap.spec.ts --project=e2e-tx   # strict E2E — see AGENTS_E2E_HYBRID_SWAP.md (#193)
make verify-issue-238   # on-chain: deployed pair/router accept `trader`; sim==execute for a discounted wallet; indexer route/solve trader-aware (needs fresh deploy + indexer; see AGENTS_QA_DEPLOY_VERIFY.md)
```

> **Post-deploy parity (L8 / #238):** the contract fix only holds end-to-end if
> the **deployed** wasm accepts `trader`. After any redeploy that changes the
> pair/router schema, run `make verify-issue-238` (script:
> [`scripts/qa/verify-issue-238.sh`](../scripts/qa/verify-issue-238.sh)) to prove
> sim==execute on-chain — stale on-chain wasm previously blocked verification.

## Related frontend skills

- [`AGENTS_E2E_HYBRID_SWAP.md`](./AGENTS_E2E_HYBRID_SWAP.md) — Playwright hybrid tx path without conditional skips ([#193](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/193))
- [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) — route spread preflight uses per-hop `hybrid_simulation` only.
