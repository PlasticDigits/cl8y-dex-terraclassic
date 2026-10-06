# Economically unfillable limit orders (#1399)

Use this playbook when changing placement, repricing, ladders, matching, cleaning,
parked refunds, or indexer book simulation. This implements the source continuation
of [#1225](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1225) in
[#1399](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1399).

## Invariants

| ID | Required behavior |
| --- | --- |
| UF1399-1 | Keep human price bounds decimal-aware (0–18 decimals); separately evaluate raw economic fillability with checked arithmetic. Human price bounds and the 10-unit size floor alone are insufficient. |
| UF1399-2 | An ask's full remaining token0 must be capable of a positive token1 payment. A bid's full remaining token1 must buy positive token0 with positive token1 cost using the contract's rounded reciprocal math. Bid full capacity must fit `Uint128`; wide ask notional may exceed it if partial fills remain usable. Invalid arithmetic is never a panic or free fill. |
| UF1399-3 | Validate every single/batch rung after maker fees, every expanded ladder rung, and the remaining escrow at reprice. Economic rejection is atomic; insertion-step skip/refund does not excuse an invalid rung. |
| UF1399-4 | Classify against the entire order, never the current taker's budget. Preserve zero-cost partial-fill skips for valid orders that a small taker cannot fill. |
| UF1399-5 | Legacy invalid live rows park as `reason=unfillable` during bounded matching or permissionless cleaning, including default zero clean thresholds. Use the existing claim map and owner-only refund path. Time expiry takes precedence over economic reason. |
| UF1399-6 | Parking unlinks the DLL and preserves pending escrow until claim. No maker transfer occurs during parking; cancellation cannot refund a parked row twice. Valid rows retain price/time FIFO and only owner repricing changes their price. |
| UF1399-7 | Keep the 500-step scan, 100-maker cap and shared 15 pre-fill expiry/blacklist/unfillable park cap. Every visited row counts. Clean pagination retains its independent scan/park caps and resumes at the first unvisited row. Repeated successful bounded calls clear finite invalid prefixes; a reverted swap cannot persist cleanup. |
| UF1399-8 | Execute and read-only contract/indexer simulation share economic classification, fixed-point rounding and scan limits for the same snapshot. Simulation skips rows without mutating or optimistically removing their scan cost. |
| UF1399-9 | Preserve pause, blacklist, token code-id freeze, slippage and transaction rollback protections. This change does not alter greedy defaults or bypass swap signing protections. |
| UF1399-10 | Record executed PASS/FAIL and unavailable SKIP separately. Overall agent verification may be PASS WITH SKIPS only with no executed failures; source tests do not establish deployment acceptance. |

## Agent instructions

1. Read [the QA checklist](../docs/qa/issue-1399/README.md),
   [limit-order behavior](../docs/limit-orders.md#unfillable-in-band-floor-orders-forgejo-1225),
   and security invariant [L26](../docs/contracts-security-audit.md).
2. Run `make verify-issue-1399`; perform runnable checks before recording skips.
3. Review ask/bid 6/6, 6/18 and 18/6 boundaries, reciprocal overflow, post-fee
   batch/ladder rejection, repricing rollback, long prefixes and escrow claims.
4. Retest after corrections. Record revision, UTC timestamp, commands, results,
   synthetic evidence, and the exact reason for every unavailable check.
5. An unavailable item can be SKIP and overall PASS WITH SKIPS. Never relabel an
   executed failure as SKIP or claim live migration from unit/DB tests. Provision
   available local dependencies first; avoid public-chain attack fixtures.

Source: [shared validation](../smartcontracts/packages/dex-common/src/limit_placement.rs),
[pair placement](../smartcontracts/contracts/pair/src/limit_placement.rs),
[contract/reprice](../smartcontracts/contracts/pair/src/contract.rs),
[matcher](../smartcontracts/contracts/pair/src/orderbook.rs),
[cleaner](../smartcontracts/contracts/pair/src/limit_book_clean.rs),
[indexer mirror](../indexer/src/api/db_orderbook_sim.rs).
Related: [batch/ladder](./AGENTS_LIMIT_ORDER_BATCH_LADDER.md),
[park reasons](./AGENTS_EXPIRED_LIMIT_PARK_REASON.md),
[greedy matching](./AGENTS_GREEDY_BOOK_FIRST.md).
