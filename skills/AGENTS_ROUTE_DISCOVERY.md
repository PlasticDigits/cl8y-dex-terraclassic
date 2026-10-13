# Bounded automatic route discovery (#1489)

Apply to skeleton enumeration, exact-amount deeper discovery, producer revision
invalidation, job contention, progress and Swap/Trade adoption. Issue:
[#1489](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1489).
Follow [the verification checklist](../docs/qa/issue-1489/README.md) and
[the search contract](../docs/route-solver.md#automatic-discovery-1489).
Run `make verify-issue-1489` and the applicable related verifiers below.

| ID | Required behavior |
| --- | --- |
| RD1489-1 | Preserve a useful short-path first result; rank every evaluated eligible improvement by exact delivered net output for the exact amount and supported policy. Never prefer an address, symbol, displayed percentage or TVL. |
| RD1489-2 | Successful unchanged producer validation must not revoke compatible original economics. Actual changed fee, negative/cancelled required reads, timeout, reserve/book/configuration/reference/wrapper/freeze changes still invalidate unsafe results. Never relabel an old cohort or renew its original expiry. |
| RD1489-3 | Bound active work and deferred scheduling; identical exact intents coalesce. Capacity release must give deferred work a finite retry opportunity. Cache hits cannot permanently strand useful short results. Expired, cancelled or invalidated work cannot publish. |
| RD1489-4 | Preserve four pool hops, 1,024 paths and 16,384 edge visits. Native wrapping is a separate message. Report truncated bounded discovery truthfully; a best evaluated candidate is not proof of a global optimum. |
| RD1489-5 | Solve, best and progress bind the same original native identity, gross amount, spread/protection, mode, actor/tier and generation. Declared POST splits retain their own constraints; receive sizing retains its documented minimum-input objective and bounded inverse scope. |
| RD1489-6 | Frontend adoption watches eligible delivered-net improvements as well as newly published generations. An unchanged generation can contain a better result. Progress is advisory, never an execution permission. Reviewed transactions remain pinned during consent/signing. |
| RD1489-7 | One factual retail notice describes selected-route economics. It cannot imply that all eligible routes have insufficient liquidity. Keep exact 5/30/99 controls, unknown-budget refusal, offline preparation and original reference quality. |
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
