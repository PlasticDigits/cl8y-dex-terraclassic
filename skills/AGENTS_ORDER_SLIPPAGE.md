# Expected order slippage (#1500)

Apply to route order metrics, cost disclosure and Swap/shared Trade risk wording.
Read the [metric/API contract](../docs/route-solver.md#expected-order-slippage-1500)
and [verification checklist](../docs/qa/issue-1500/README.md).

| ID | Invariant |
| --- | --- |
| OS1500-1 | Expected order slippage is signed `(selected fee-free spot output - selected fee-free execution output) / selected fee-free spot output`. Establish the denominator independently from original selected reserve ratios and gross principal. Never divide market movement by independent fair output. |
| OS1500-2 | Remove pool/book fees, sell/buy taxes, hooks, wrapper losses and independent-reference divergence from the counterfactual; network gas is never output loss. Favorable book prices remain negative. No summed per-hop percentage. |
| OS1500-3 | Walk only selected original mirrors, one to four hops, original selected book-budget fractions and maker caps. Replay integer Decimal book math, floor normalization and pool residue. Each stage has at most 500 actor-free price/escrow terms. |
| OS1500-4 | Bind method/version, exact original input, operations, policy/tolerance, generation, observation and expiry. Deferred publication may tighten timestamps only. Missing, malformed, overflowing or unsupported evidence is unavailable, never zero. |
| OS1500-5 | Clients bind reserve/direction evidence and the complete bounded sanitized book_terms (including original unconsumed rows) to original execution hops, independently validate actual pool gross/spread and book consumed/precommission output, chained spot baseline, fee-free pool/book output, canonical signed numerator, actual common-unit commissions and signed attribution reconciliation. Advisory metrics cannot authorize signing. |
| OS1500-6 | Trading Fee uses actual original commissions projected through suffix spot ratios and the fee-free spot denominator. Raw trading fee + tax/hook cost = original trading_cost. Detail amounts use final-output token units; gas appears once in LUNC. |
| OS1500-7 | Slippage, Fee and actual token route share the compact row; costs and accurately named reference comparison use one DETAILS disclosure. Risk notices say Fair-reference deviation and cannot attach one cost cause to its aggregate percent. |
| OS1500-8 | Preserve genuine >5% consent, >30% Expert review, >=99% nonwaivable symmetric fair/net refusal, including favorable deviations. Exact delivered net still ranks candidates; small order slippage cannot weaken original reference admission. |
| OS1500-9 | Preserve original offline observations, policy, expiry, positive post-hook/hop/final floors, route-local tax funding/refunds and unknown-budget signing refusal. Native fee removal assumes only the admitted equal-raw-principal mapping; unsupported wrappers cannot borrow that model. |
| OS1500-10 | Run `make verify-issue-1500` and 1483/1489/1473/1480/1452 plus applicable reference/floor/tax/launch checks. First/retest final source, retain executed failures, reset only owned affected fixtures. Source passes cannot attest UI deployment or funded/physical settlement. |

This follow-up explicitly supersedes only RC1483-2/RC1483-7's old retail
percentage/labels/placement. Preserve [RC1483](AGENTS_ROUTE_COST_ATTRIBUTION.md),
[LQ1452](AGENTS_LOCAL_SWAP_QUOTES.md), [RD1489](AGENTS_ROUTE_DISCOVERY.md),
[TQ1473](AGENTS_TRADING_QUOTE_OUTCOMES.md), [OB1480](AGENTS_OFFLINE_SWAP_BUDGET.md),
[SM1479](AGENTS_SMALL_MARKET_REFERENCE.md), [NE1398](AGENTS_NET_EXECUTION_FLOORS.md),
[RT1381](AGENTS_ROUTE_LOCAL_TAX.md), [LG1390](AGENTS_ROUTE_LAUNCH_GUARDS.md).
