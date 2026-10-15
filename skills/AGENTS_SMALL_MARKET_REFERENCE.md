# Small-market reference eligibility (#1479)

Apply to economic-mark admission, small-market quotes, policy exclusions and
pricing readiness. Issue: [#1479](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1479).
Read [the agent checklist](../docs/qa/issue-1479/README.md) and preserve
[TQ1473](AGENTS_TRADING_QUOTE_OUTCOMES.md), [LQ1452](AGENTS_LOCAL_SWAP_QUOTES.md),
[O1391](AGENTS_ORACLE_SAFETY.md), [PR1464](AGENTS_PROVIDER_RECOVERY.md),
[LF1453](AGENTS_LIQUIDITY_FEED.md), [DB1396](AGENTS_INDEXER_DATABASE_ISOLATION.md).

| ID | Invariant |
| --- | --- |
| SM1479-1 | The reporting floor measures total pair TVL. Independently priced anchor-leg depth is a separate quantity. Neither displayed positive TVL nor a catalog active flag admits execution. |
| SM1479-2 | The user authorized a $10 economic total-TVL floor on 2026-10-12. Keep the independent hub priced-leg floor at $100; use separate configuration. Diagnostic calculations below the economic floor are evidence, never fair prices or signing permission. |
| SM1479-3 | Existing economic pool marks remain provenance-bound spot-derived marks; reducing their reporting floor does not certify an independent oracle for the token. Its candidate return, a symbol match, circular unpriced backing or a new timestamp on retained data cannot establish a new independent reference class. No address exceptions or reference-free signing. |
| SM1479-4 | Publish policy evidence only from scheduled ingestion with qualified original-fresh anchors and a complete bounded candidate census. Unknown/incomplete/stale alternatives retain transient data classification. |
| SM1479-5 | Original-time policy evidence expires after 20 seconds, rejects future times and is superseded by admitted prices. It never extends the pool/book/configuration lifetimes or splices generations. Scheduled hub prices, economic marks, oracle quorum and policy census are read from one consistent read-only database snapshot. |
| SM1479-6 | A proved structural exclusion uses fixed 422 `reference_policy_excluded`; missing/expired observations remain recoverable 503. Deterministic solve errors have no retry timer; a changed scheduled generation may recover the active intent. |
| SM1479-7 | Progress search completion and executable eligibility are separate. Idle/completed search is not a quote; no request drives reference production or chain/network validation. |
| SM1479-8 | Preserve exact raw economics, taxes/hooks, actor/context, positive hop/post-hook/final floors, exact selected UI percentage and nonwaivable ≥99% protection. |
| SM1479-9 | Diagnostics expose fixed reasons and bounded counts, not provider bodies, endpoints, actors or wallet digests. Tests own disposable database/chain state. |
| SM1479-10 | Run `make verify-issue-1479`, `make verify-issue-1473` and relevant existing verifiers; fix executed failures and repeat on stable source. Explicit unavailable SKIP leaves acceptance unchecked. Diagnostics and a lower reporting threshold alone do not complete independent-reference AC3 or settlement/deployed acceptance. |

This implementation separates the explicitly authorized economic reporting floor
from the hub backing requirement and diagnoses remaining exclusions. It introduces
no new reference quality class. Existing spot-derived economic marks retain their
source/time/anchor checks; the lower threshold does not prove independent token
reference integrity under AC3. Additional historical/corroborated evidence needs
its own reviewed manipulation tests; a waiting period alone is not that evidence.

Generic route cost attribution, exact-amount cache ranking and consistent fair-reference
explanations follow [RC1483](AGENTS_ROUTE_COST_ATTRIBUTION.md) and
[the checklist](../docs/qa/issue-1483/README.md). Run `make verify-issue-1483`;
explanation never changes reference quality, funding, floors or signing admission.

Reference scheduling follows [NR1505-9](AGENTS_NATIVE_QUOTE_RECOVERY.md).
Coalesced owned pool-cycle completion may wake the existing mark worker;
accounting consumes one repeatable-read certified context independently. Preserve
the same admission rules and original timestamps: scheduling cannot promote a
new price class, replace unavailable qualification or extend expiry. Include
`make verify-issue-1505` when changing that producer wiring.
