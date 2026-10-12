# Route economic attribution (#1483)

Apply when changing route cost diagnostics, amount-specific candidate caching,
fair-reference explanations or Swap/Trade review. Issue:
[#1483](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1483).
Implementation: [PR #1484](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1484).
Read [the rounding/API contract](../docs/route-solver.md#economic-attribution-1483)
and [the agent checklist](../docs/qa/issue-1483/README.md).
Run `make verify-issue-1483` and the applicable existing verifiers listed there.

| ID | Invariant |
| --- | --- |
| RC1483-1 | All supported identities and exact entered amounts use generic checked economics. A small-amount cached winner cannot exclude a better eligible large-amount candidate. Every bounded evaluated strictly higher delivered-net deeper result is eligible for publication, including improvements below 0.1%; reviewed transactions remain pinned. Never scale an executable cached quote or prefer an address, symbol or smaller displayed deviation. |
| RC1483-2 | The only retail percentage remains symmetric fair-reference deviation: `100 * abs(fair - net) / max(fair, net)`. Preserve favorable-deviation warnings and all >5%, >30% and >=99% safeguards. Attribution cannot weaken execution admission. |
| RC1483-3 | `spot_bridge_v1` projects original per-hop actual stage boundaries through remaining fee-free original reserve spot ratios into raw final-output units. Differences telescope exactly: wrapper + trading + market movement + reference divergence = fair - delivered net. Components are signed integers, never additive per-hop percentages. |
| RC1483-4 | Wrapper attribution uses authorized gross input versus actual credited input and final actual unwrap loss once. Network gas/fee remains external to swap output, protected principal and references. Preserve native pool-only admission and unsupported unwrap blocks. |
| RC1483-5 | Pool commissions, book taker commissions, sell/buy taxes, residue and supported hooks use their actual observed stage boundaries. Signed book improvement and favorable reference offsets remain signed; equal components or offsetting causes yield truthful mixed explanations. |
| RC1483-6 | All attribution stages bind original generation, observation and expiry. Missing/inconsistent/overflow evidence yields unavailable attribution, never zero cost, an executable fallback or renewed time. Original reference quality and independent $10 economic/$100 hub-depth floors remain unchanged. |
| RC1483-7 | Diagnostics use bounded fixed schemas and public integer fields only. No actors, context digests, provider text, arbitrary upstream enum or private endpoint appears in retail copy/public receipts. Client explanation derives from reconciled fields bound to the current quote; it never supplies signing permission. |
| RC1483-8 | Quote/input/refresh/review/preparation remains offline. Scheduled ingestion owns observations. Preserve bounded route/hop/work/cache/concurrency limits, positive upward-rounded floors, exact gross route funding, actor/launch restrictions and unknown-budget signing refusal. |
| RC1483-9 | Independent fixtures compare native gross, matching wrapped credit and equal-gross wrapped separately on one immutable cohort. Compare eligible direct/UST1/CL8Y paths naturally, solve/best/POST and supported receive sizing; record genuine high-impact best routes without manufacturing lower warnings. |
| RC1483-10 | First/retest stable final source, reset only owned affected fixtures after failures, and retain original failed inputs. Attest backend/frontend/contract artifacts separately; initialized socket <=500ms and browser <=750ms use original useful-output clocks. Executed failures remain FAIL; skips remain unchecked and cannot establish deployment/settlement or close the issue. |

Implementation: [backend attribution](../indexer/src/api/local_quote/attribution.rs),
[exact book commission stages](../indexer/src/api/db_orderbook_sim.rs),
[same-cohort ranking fixture](../indexer/tests/scenarios/route_cost_attribution_1483.rs),
actual [Swap/Trade explanation](../frontend-dapp/src/components/swap/SwapImpactAside.tsx).
Preserve [LQ1452](AGENTS_LOCAL_SWAP_QUOTES.md),
[TQ1473](AGENTS_TRADING_QUOTE_OUTCOMES.md),
[SM1479](AGENTS_SMALL_MARKET_REFERENCE.md),
[OB1480](AGENTS_OFFLINE_SWAP_BUDGET.md),
[NE1398](AGENTS_NET_EXECUTION_FLOORS.md),
[RT1381](AGENTS_ROUTE_LOCAL_TAX.md),
[DB1396](AGENTS_INDEXER_DATABASE_ISOLATION.md).

The reported mobile input, original cohort/artifact and device/wallet context are
unavailable. Newly chosen public or synthetic inputs do not reproduce the
historical report. Current spot attribution describes an arithmetic bridge, not
a new independent oracle or proof of manipulation-free markets. See
[results and remaining limits](../docs/qa/issue-1483/RESULTS.md).
