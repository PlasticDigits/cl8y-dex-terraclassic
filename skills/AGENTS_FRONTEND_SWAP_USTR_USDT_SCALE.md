# Agent playbook: USTR→USDT quote scale (Forgejo #1257)

Audience: third-party agents touching indexer hop sim, `GET /route/solve` cache keys, Swap / Trade **You Receive**, or Expert Mode slippage.

**Issue:** [Forgejo **#1257**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1257)  
**Invariants:** [`docs/frontend.md` § USTR/USDT quote scale](../docs/frontend.md#ustr-usdt-quote-scale) (**Q1257-1–Q1257-8**)  
**Indexer:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) mixed-decimal hop honesty · [`docs/route-solver.md`](../docs/route-solver.md) `AMOUNT_CACHE_BUCKET`  
**UX:** [`docs/swap-max-spread-ux.md`](../docs/swap-max-spread-ux.md) invariant **7** (Expert does not waive ≥99%)

## Problem class

USTR and USDT are **18-dec**; UST1 / cLUNC / cUSTC are **6-dec**. A 3-hop USTR→UST1→cLUNC→USDT path plus `u128` saturating `k` (or 18-dec offer into a 6-dec last hop) printed a near-full drain. Swap `getDecimals` unknown→6 then showed `9.091e15` raw as **9.091B**. Retail sizes 1 / 100 / 1000 / 10000 looked non-monotonic because the solver switched onto that theater path. Unique-symbol USDT is `terra1z0xe7…` (tokenlist).

## Invariants (Q1257)

| ID | Rule |
|----|------|
| **Q1257-1** | Mixed 18/6 hop sim uses **wide** `k` (pair Uint256 analog, #464). `saturating_mul` overflow is not a quote. |
| **Q1257-2** | Hop is unusable (`ImplausibleHop` → skip path) when ask_out **>** ask reserve, **or** offer > **1000×** input reserve **and** ask_out ≥ **99%** of ask reserve. Do **not** 404 an honest 1-hop whale 99% drain. |
| **Q1257-3** | `hybrid_cache_key` includes `token_in` / `token_out`. `AMOUNT_CACHE_BUCKET` = 1e6 raw: 1 USTR (`1e18`) and 10000 USTR (`1e22`) must not alias; 1 USTR must not alias 1 UST1 (`1e6`). |
| **Q1257-4** | Unique-symbol USDT (`terra1z0xe7…`) and USTR display as **18** decimals. Unknown CW20 stays **6** on `getDecimals` / `swapAmountDecimals` chrome only. Swap / Trade **execute** amounts use `useAssetDecimals` (`null`, not 6) ([#1255](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1255)). Do not globally remove the unknown default. |
| **Q1257-5** | Swap / Trade hide **You Receive** (show `—` / empty, not billions) when expected slippage **≥ 99%**. |
| **Q1257-6** | Expert Mode waives **> 30%** only. **≥ 99%** stays blocked; do not offer Expert as an escape hatch. |
| **Q1257-7** | Honest mixed-dec pools stay size-monotonic on per-human-unit out (larger size → ≤ per-unit). Theater 3-hop must lose to a funded 1-hop when both exist. |
| **Q1257-8** | Tokenlist USDT ticker stays unique (ASCII fold). Do not LCD-fetch `token_info.symbol` for display scale. |

## Canonical code

| File | Role |
|------|------|
| `indexer/src/api/db_orderbook_sim.rs` | `simulate_pool_leg` wide k, `hop_sim_implausible` |
| `indexer/src/api/hybrid_route_opt.rs` | `ImplausibleHop` → `PathUnusable` |
| `indexer/src/api/route_solver.rs` | `amount_cache_key` / `hybrid_cache_key` |
| `frontend-dapp/src/utils/swapQuoteAmountScale.ts` | decimals pin + theater submit guard |
| `frontend-dapp/src/pages/SwapPage.tsx` | hide receive + Expert no-waive ≥99% |
| `frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx` | same display + submit block |

## Do / don’t

- **Do** skip implausible hops so a shorter funded path can win.
- **Do** keep Expert for 30–99% (thin-book honesty).
- **Don’t** treat ≥99% as a confirm-again. **Don’t** 404 every 99% hop.
- **Don’t** change pair wasm `k` here — indexer only; chain already uses Uint256 (#464).
- **Don’t** map a second USDT ticker. **Don’t** use `parseFloat` on 18-dec raw.

## Regression

```bash
make verify-issue-1257
```

Vitest: `swapQuoteAmountScale.test.ts`, `SwapPage.test.tsx` (30% Expert waive + 99% theater).  
Indexer lib: `db_orderbook_sim` mixed 18/6 + implausible; `route_solver` amount-bucket.

## Related

- [`AGENTS_FRONTEND_SWAP_TOKENLIST_SYMBOLS.md`](./AGENTS_FRONTEND_SWAP_TOKENLIST_SYMBOLS.md) — unique USDT ticker (**TL-1**)
- [`AGENTS_HYBRID_QUOTING.md`](./AGENTS_HYBRID_QUOTING.md) — quote = execute
- [`AGENTS_FRONTEND_SWAP_AMOUNT_SCALE.md`](./AGENTS_FRONTEND_SWAP_AMOUNT_SCALE.md) — unlisted CW20 execute scale (**Q1255**)
- [`AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md`](./AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md) — do not weaken 30/99 gates
- Remaining GET `/route/solve` failures census (**Stay**): [ADR 0007](../docs/adr/0007-route-solve-remaining-failures.md) / [#1265](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1265). **F1** stays this ticket. Verify: `make verify-issue-1265`.
