# Bounded automatic route discovery (#1489)

Implementation: [PR #1490](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1490).

Apply to skeleton enumeration, exact-amount deeper discovery, producer revision
invalidation, job contention, progress and Swap/Trade adoption. Issue:
[#1489](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1489).
Follow [the verification checklist](../docs/qa/issue-1489/README.md) and
[the search contract](../docs/route-solver.md#automatic-discovery-1489).
Run `make verify-issue-1489` and the applicable related verifiers below.

| ID | Required behavior |
| --- | --- |
| RD1489-1 | Preserve a useful short-path first result; rank every evaluated eligible improvement by exact delivered net output for the exact amount and supported policy. Never prefer an address, symbol, displayed percentage or TVL. |
| RD1489-2 | Successful unchanged producer validation must not revoke compatible original economics. Hash the effective qualified market after preserving newer views and filtering rejected views; identical effective observations must not acquire history-dependent generations or depend on HashMap insertion order. Actual changed fee, negative/cancelled required reads, timeout, reserve/book/configuration/reference/wrapper/freeze changes still invalidate unsafe results. Capture actor guard proofs when the authoritative blacklist observation completes, before optional discount/enrichment awaits; reconcile late admission against the bounded authoritative ledger. False guard responses bind query-start authority; an older response cannot clear a newer known/inventory block. Repeated already-unblocked refreshes retain fresh verified fees without renewing deadlines. Latest-started fee reads own completion; superseded publications drop and cancelled current reads revoke fees conservatively. Fresh unblocks keep conservative fees until new fee evidence completes, and recycled actor records cannot reuse an old proof. Complete inventory resets revoke omitted per-actor discount proofs, including late admission without an expected configuration epoch; repeated unchanged inventories retain stable versions. Never relabel an old cohort or renew its original expiry. |
| RD1489-3 | Bound active work and deferred scheduling; identical exact intents coalesce. Capacity release must give deferred work a finite retry opportunity. Cache hits cannot permanently strand useful short results. Expired, cancelled or invalidated work cannot publish. |
| RD1489-4 | Preserve four pool hops, 1,024 paths and 16,384 edge visits. Native wrapping is a separate message. Report truncated bounded discovery truthfully; a best evaluated candidate is not proof of a global optimum. |
| RD1489-5 | Solve, best and progress bind the same original native identity, gross amount, spread/protection, mode, actor/tier and generation. Declared POST splits retain their own constraints; receive sizing retains its documented minimum-input objective and bounded inverse scope. |
| RD1489-6 | Frontend adoption watches eligible delivered-net improvements as well as newly published generations. An unchanged generation can contain a better result. Progress is advisory, never an execution permission. Reviewed transactions remain pinned during consent/signing. |
| RD1489-7 | One factual retail notice describes selected-route economics. It cannot imply that all eligible routes have insufficient liquidity. When a same-intent unexpired quote and receive amount remain displayed during background refresh, retain that quote's notice while conservative signing stays blocked. Clear old-intent/expired notices; update thresholds only when a newly validated quote changes. Never suppress genuine >5% changes or add hysteresis across 5%. An empty/zero amount, debounce, unresolved positive local quote or expired prior wrapper context does not prove wrapper unavailability. Keep entry neutral and show truthful pricing/calculating states; impossible dust/precision and terminal failures retain explicit refusals. Known positive cached pause/treasury mismatch and unknown budgets remain blockers. Keep exact 5/30/99 controls, unknown-budget refusal, offline preparation and original reference quality. |
| RD1489-8 | Preserve gross authorization, actual wrapper credit, signed raw-output attribution, upward-rounded positive hop/final floors, taxes/hooks, blacklist/pause/freeze, capable-asset repetition and owned refunds. Network fees remain separate. |
| RD1489-9 | Quote/progress/review requests read initialized observations and trigger zero RPC. Independently scheduled ingestion owns observations. Tests own disposable databases and fixtures; never reset production or shared infrastructure. |
| RD1489-10 | Retain failed inputs and actual failure verdicts. Repeat stable final source checks, separately attest backend/frontend/root/contracts, and run sustained API/browser checks beyond 20s/180s. Socket <=500ms and browser <=750ms retain original useful-result clocks. Explicit SKIP leaves acceptance unchecked; HTTP200, source tests or generation equality alone never prove settlement. |

Preserve [RC1483](AGENTS_ROUTE_COST_ATTRIBUTION.md),
[LQ1452](AGENTS_LOCAL_SWAP_QUOTES.md), [PR1464](AGENTS_PROVIDER_RECOVERY.md),
[TQ1473](AGENTS_TRADING_QUOTE_OUTCOMES.md),
[SM1479](AGENTS_SMALL_MARKET_REFERENCE.md),
[OB1480](AGENTS_OFFLINE_SWAP_BUDGET.md),
[DB1396](AGENTS_INDEXER_DATABASE_ISOLATION.md),
[NE1398](AGENTS_NET_EXECUTION_FLOORS.md),
[RT1381](AGENTS_ROUTE_LOCAL_TAX.md) and [LG1390](AGENTS_ROUTE_LAUNCH_GUARDS.md).
Related verification: `1483`, `1452`, `1473`, `1479`, `1480`, `1464`, `1453`,
`1391`, `1398`, `1381`, `1390`, `1396`.

The neutral native entry fix and directed checks are documented in
[EMPTY-ENTRY.md](../docs/qa/issue-1489/EMPTY-ENTRY.md).

Neutral entry implementation: [PR #1492](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1492).

Canonical publication, bounded preparation and actor-proof safety are documented
in [the current source supplement](../docs/qa/issue-1489/RESULTS.md#final-canonical-publication-source-supplement)
and [its fourteen-verifier receipt](../docs/qa/issue-1489/core-final-summary.json).

Canonical publication implementation: [PR #1493](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1493).

Bounded browser lifecycle and first-failure diagnostics: [BROWSER-LIFECYCLE.md](../docs/qa/issue-1489/BROWSER-LIFECYCLE.md). Original caps/clocks remain mandatory; incomplete live checks remain unchecked.
