# Agent playbook: Batch / ladder limit orders (GitLab #206)

Use when changing **multi-rung limit placement** on-chain, in the indexer, or in the dApp (`/limits` ladder panel, gas preflight, E2E).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/limit-orders.md § Place / cancel limit](../docs/limit-orders.md#place--cancel-limit-gitlab-206) | Messages, partial book-walk, batch attrs |
| [docs/limit-orders.md § Batch / ladder gas savings](../docs/limit-orders.md#batch-ladder-gas-savings) | Gas model vs N separate placements |
| [`dex-common` `limit_placement.rs`](../smartcontracts/packages/dex-common/src/limit_placement.rs) | Ladder expansion (`equal` only) |
| [`pair` `limit_placement.rs`](../smartcontracts/contracts/pair/src/limit_placement.rs) | Batch execute, refund, per-rung attrs |
| [`pair.rs`](../smartcontracts/packages/dex-common/src/pair.rs) | `Cw20HookMsg::PlaceLimitOrderBatch` / `PlaceLimitOrderLadder` |
| [`terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts) | `gasLimitForLimitOrderBatch`, batch base + per-rung |
| [`transactions.ts`](../frontend-dapp/src/services/terraclassic/transactions.ts) | `estimateLimitOrderBatchPlaceSequenceUlunaFeesTotal` |
| [`LimitOrderLadderPanel.tsx`](../frontend-dapp/src/components/trade/LimitOrderLadderPanel.tsx) | Ladder UI + gates |
| [`limitOrderNonCrossing.ts`](../frontend-dapp/src/utils/limitOrderNonCrossing.ts) | Post-only crossing guard per rung ([#297](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/297), [#385](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/385); book head + tape/pool ref fallback, same parity target as [`TradeOrderTicket`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx)) |
| [`useLimitLadderPlaceGates.ts`](../frontend-dapp/src/hooks/useLimitLadderPlaceGates.ts) | Escrow + LUNC preflight for total escrow |
| [`useLimitOrderEscrowBalance.ts`](../frontend-dapp/src/hooks/useLimitOrderEscrowBalance.ts) | CW20 escrow balance (`tokenBalance` query key; ladder gates import this directly) |
| [`useTokenBalance.ts`](../frontend-dapp/src/hooks/useTokenBalance.ts) | Re-export alias of `useLimitOrderEscrowBalance` ([#231](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/231)) |
| [`limitOrderLadder.ts`](../frontend-dapp/src/utils/limitOrderLadder.ts) | Client ladder preview + `sumLadderAmountsRaw` (must match `expand_limit_ladder`) |
| [`indexer/parser.rs`](../indexer/src/indexer/parser.rs) | One `limit_order_placements` row per `action=place_limit_order` |

## Invariants

1. **One side per batch** — bid uses token1 CW20; ask uses token0. Mixed sides → separate txs.
2. **CW20 `send` amount** = sum of per-rung gross `amount` fields (before maker fee split on-chain).
3. **One allowance** covers total escrow for the batch/ladder path. Aggregate with [`sumLadderAmountsRaw`](../frontend-dapp/src/utils/limitOrderLadder.ts) (`reduce` + `BigInt`, initial `0n`) — **never** string-concatenate rung amounts in reduce (invalid Uint128; [#233](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/233)).
4. **Validation errors** revert the whole tx; **`LimitInsertStepsExceeded`** on a rung skips that rung, refunds its escrow, continues; **zero** successful rungs → `LimitBatchNoRungsPlaced`.
5. **Indexer** scans every `action=place_limit_order` in the wasm stream (including after `place_limit_order_batch`); do not use `wasm_attr_last` for placement parsing ([#141](../docs/limit-orders.md)). On-chain batch txs emit **columnar** attrs (all `action`s first, then parallel `order_id` / `price` columns); `parse_limit_order_placements_columnar` in [`parser.rs`](../indexer/src/indexer/parser.rs) zips them — interleaved attrs remain supported in tests.
6. **Gas preflight** for ladder uses `estimateLimitOrderBatchPlaceSequenceUlunaFeesTotal(rungCount)` — keep aligned with `getGasLimitForTx` for `place_limit_order_batch` / `place_limit_order_ladder` ([#132](./AGENTS_TERRACLASSIC_GAS.md)).
7. **Retail single** order uses batch with one item ([`placeLimitOrderWithAllowance`](../frontend-dapp/src/services/terraclassic/pair.ts)).
8. **Escrow balance hook must resolve** — [`useLimitLadderPlaceGates`](../frontend-dapp/src/hooks/useLimitLadderPlaceGates.ts) imports [`useLimitOrderEscrowBalance`](../frontend-dapp/src/hooks/useLimitOrderEscrowBalance.ts) (not a missing `@/hooks/useTokenBalance` module). [`useTokenBalance`](../frontend-dapp/src/hooks/useTokenBalance.ts) is only a re-export for callers that want a generic name; do not duplicate `useQuery` logic ([#231](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/231)).
9. **Batch storage collapse ([#247](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/247))** — `execute_place_limit_orders_batch` must use one `ORDER_NEXT_ID` write per batch and one `PENDING_ESCROW_*` write per token side touched; order id sequence must match sequential singles (`batch_placement_order_ids_match_sequential_singles`). Helpers: `reserve_order_id_block`, `insert_*_with_id(..., update_escrow: false)` in [`orderbook.rs`](../smartcontracts/contracts/pair/src/orderbook.rs).
10. **Batch hint chaining ([#256](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/256))** — each rung passes the prior **successfully placed** rung id as `hint_after` for O(1) insert when ladder prices are monotonic in book order; invalid hints fall back to head walk; **near-miss** hints on valid anchors walk from the hint toward head/tail ([#265](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/265)). **Per-rung wire ([#261](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/261)):** optional **`hint_after_order_id`** on each `LimitOrderPlacementItem` wins over internal chaining. Resolve predecessors via indexer **`insert-hints`** ([#267](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/267)) — not LCD. Clients may also supply hints on `UpdateLimitOrderPrice`. Invariant **L14** in [contracts-security-audit.md](../docs/contracts-security-audit.md); [integrators.md § Insert hints & price window](../docs/integrators.md#insert-hints-price-window-gitlab-267).
11. **Book-order batch traversal ([#266](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/266))** — `execute_place_limit_orders_batch` assigns ids by **input index**, sorts inserts by composite book key (`bid_before` / `ask_before`), threads **`InsertThreadCursor`** between successful rungs for O(0)-load interior verify, and emits `place_limit_order` attrs in **input/id order** (indexer columnar zip unchanged). **`LimitOrderLadderSpec.hint_after_order_id`** (optional, `#[serde(default)]`) applies to the **boundary** (head-most) rung only; client resolves via indexer **#267**. Invariants **L12** / **L14**; companion frontend **#268**.
12. **Deep-book ladder dApp ([#268](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/268))** — before submit, probe indexer **`limit-book?price_from&price_to`** + **`insert-hints`**; choose **`thin_ladder`** | **`single_anchor_ladder`** | **`deep_batch`** (hinted batch). **`resolved:false`** → omit hint (never fabricate id). Adaptive **`max_adjust_steps`** default from [`limitLadderAdaptiveSteps.ts`](../frontend-dapp/src/utils/limitLadderAdaptiveSteps.ts) (floor **32**, cap **256**). Indexer outage → degrade to conservative ladder + warning. Code: [`useLimitLadderPlacementPlan`](../frontend-dapp/src/hooks/useLimitLadderPlacementPlan.ts), [`limitLadderPlacementPlan.ts`](../frontend-dapp/src/utils/limitLadderPlacementPlan.ts), [`LimitOrderLadderPanel.tsx`](../frontend-dapp/src/components/trade/LimitOrderLadderPanel.tsx). Disable single-anchor on legacy wasm: `VITE_LIMIT_LADDER_SINGLE_ANCHOR=false`.
13. **Ladder post-only crossing guard ([#297](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/297), [#385](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/385))** — before broadcast, each ladder rung must pass the same client-only check as the retail limit form: **`describeLimitCrossingBlockerWithRef(side, price, bestBid, bestAsk, refToken1PerToken0)`** from [`limitOrderNonCrossing.ts`](../frontend-dapp/src/utils/limitOrderNonCrossing.ts) with head rows from [`useTradeBestBookPrices`](../frontend-dapp/src/hooks/useTradeBestBookPrices.ts) (also seeds from cached `limitBookPage` infinite queries). Book-head crossing is checked first; when the opposite side of the book is empty, the guard falls back to the indexed tape / AMM pool reference (`refToken1PerToken0` from [`useLimitOrderPriceRefBundle`](../frontend-dapp/src/hooks/useLimitOrderPriceRefBundle.ts)) so BID ladders above market still block when `best_ask` is missing. When any rung crosses, disable submit, show inline **`N of M rungs will cross the market…`** via [`LimitOrderEscrowPlaceGuardMessage`](../frontend-dapp/src/components/trade/LimitOrderEscrowPlaceGuardMessage.tsx) (`data-testid="ladder-crossing-guard"`), and re-check in `mutationFn`. The pair accepts crossing limits on-chain by design ([#152](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/152)); this is UX-only. Vitest: [`limitOrderNonCrossing.test.ts`](../frontend-dapp/src/utils/__tests__/limitOrderNonCrossing.test.ts), [`LimitOrderLadderPanel.crossing.test.tsx`](../frontend-dapp/src/components/trade/__tests__/LimitOrderLadderPanel.crossing.test.tsx).

## Tests to run after changes

```bash
# Contracts (batch + partial + ladder)
cd smartcontracts && cargo test -p cl8y-dex-tests limit_batch place_limit_order_ladder -- --nocapture

# Frontend unit
cd frontend-dapp && npm test -- limitOrderLadder limitOrderNonCrossing limitOrderBatchGasSummary useLimitLadderPlaceGates useTokenBalance limitLadderBoundary limitLadderAdaptiveSteps limitLadderDepth limitLadderPlacementPlan useLimitLadderPlacementPlan
# Deep-book ladder (#268)
cd frontend-dapp && npm test -- limitBookInsertHint LimitOrderLadderPanel
# Ladder crossing guard (#297)
cd frontend-dapp && npm test -- LimitOrderLadderPanel.crossing limitOrderNonCrossing
# Regression: sumLadderAmountsRaw must not string-concat rungs (#233)
cd frontend-dapp && npm test -- limitOrderLadder -t "GitLab #233"

# E2E (LocalTerra, 5 workers)
bash scripts/e2e-provision-dev-wallet.sh
cd frontend-dapp && npx playwright test e2e/limit-orders-tx.spec.ts --project=e2e-tx
```

## Related skills

- Limit place/cancel E2E: [`AGENTS_E2E_LIMIT_ORDERS_TX.md`](./AGENTS_E2E_LIMIT_ORDERS_TX.md)
- Placement gas presets: [`AGENTS_FRONTEND_LIMIT_ORDER_PLACEMENT_GAS.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PLACEMENT_GAS.md)
- Terra gas / two-tx sequences: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)

## GitLab

- [#206](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/206) — batch / ladder feature + verification checklist
- [#266](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/266) — book-order traversal, thread cursor, ladder anchor hint (this playbook §11)
- [#267](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/267) — indexer insert-hint API (ladder anchor source)
- [#268](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/268) — frontend deep-book ladder (depends on #266 + #267)
- [#231](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/231) — missing `useTokenBalance` broke Vite after clean `node_modules` (ladder gates → `useLimitOrderEscrowBalance`)
- [#233](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/233) — `sumLadderAmountsRaw` string-concat broke `increase_allowance` Uint128; fixed in `515fba3`, regression test + docs crosslink
- [#297](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/297) — ladder panel missing post-only crossing guard (ported from `TradeOrderTicket`; MR !39)
- [#385](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/385) — BID ladder crossing guard skipped when `best_ask` empty (reference fallback + `limitBookPage` cache seed)
