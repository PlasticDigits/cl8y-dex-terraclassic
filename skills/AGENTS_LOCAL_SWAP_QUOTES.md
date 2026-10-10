# Local swap quotes (#1452)

Implementation: [merged PR #1455](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1455). Current implementation: [follow-up PR #1458](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1458).


Apply when changing route discovery, quote caches, liquidity ingestion, reference
pricing, refreshes, preflight or Swap/Trade signing. Issue:
[#1452](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1452).
Read the [current solver contract](../docs/route-solver.md#local-quote-contract-1452)
and [agent verification checklist](../docs/qa/issue-1452/README.md).

| Invariant | Required behavior |
| --- | --- |
| LQ1452-1 | Initial results, deeper search, references, refreshes and reviewed-route preparation read initialized local observations. No request may trigger chain polling, attestation, router simulation, config/tier discovery or a remote fallback. |
| LQ1452-2 | An independently scheduled, nonoverlapping worker maintains bounded observations. Missing, stale, future-dated or unsupported required semantics fail explicitly. Preserve original observation times on ingestion failure; a successful read does not make other failed reads fresh. Coordinate independent reserve/book generations with [#1453](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1453). |
| LQ1452-3 | Search direct and one-intermediate paths before bounded longer search. Rank amount-specific delivered net output subject to the caller's actual spread constraint. Contract identity and checked integer economics determine eligibility; symbols, address order, USD marks and raw cross-decimal reserves do not determine preferences. |
| LQ1452-4 | Useful first output precedes deeper work. A loading/progress acknowledgement is not a quote. The current temporary target is ≤500 ms separately for initialized route misses and hits; the original <250 ms target is deferred by explicit user direction in #1464. Measure useful results, including socket transport and browser input; cold initialization and production deployment are separate evidence. |
| LQ1452-5 | Bound graph size, edge visits, mirror/book work, exact-input results, skeletons, references, background jobs and concurrent requests. Identical requests coalesce deeper work; cancellation and worker errors release resources. Cache keys bind namespace, observation generation, direction, exact offer/target, mode, actor/tier, spread, maker cap and funding/tax semantics. Never linearly scale an executable cached quote. |
| LQ1452-6 | Output, references, impact, per-hop positive floors and final minimum receive come from compatible observations. Unavailable fair references are unavailable, never zero impact. Cache hits never inherit old enrichment and relabel it fresh. L9 spread and fair-reference deviation remain separate. |
| LQ1452-7 | Confirmation pins input, route, operations, floors, constraints and generation. Stale intent and context changes block mutation. Refresh or deeper improvement cannot silently replace the reviewed transaction. Keep 30% Expert behavior and the nonwaivable 99% guard. |
| LQ1452-8 | Preserve [NE1398 net floors](AGENTS_NET_EXECUTION_FLOORS.md), [RT1381 gross funding](AGENTS_ROUTE_LOCAL_TAX.md), [LG1390 launch guards](AGENTS_ROUTE_LAUNCH_GUARDS.md), freeze/blacklist/pause exclusions, gems and wallet debit bounds. Unsupported models cannot become gross or wallet-funded fallback quotes. |
| LQ1452-9 | Keep the retail flow compact and automatic under [copy rules](AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md). One applicable blocker/action. Never suggest pay/10 without a fresh smaller quote proving it satisfies the actual constraint. Technical provenance belongs in backend/test evidence. |
| LQ1452-10 | Run `make verify-issue-1452`, inspect meaningful assertions, retest corrections and record exact revision, UTC time, counts, generations and latency distributions/maxima. An unavailable check may be explicit SKIP with overall PASS WITH SKIPS when executed checks pass. A failed assertion stays FAIL; skipped acceptance remains unchecked and source tests never establish deployed acceptance. |

Runtime database access remains least privilege; integration processes own
[disposable databases](AGENTS_INDEXER_DATABASE_ISOLATION.md). Never mutate
production reserves/books or publish private reporter/actor data for QA.

## Implementation map

- [Local engine and generation cache](../indexer/src/api/local_quote.rs),
  [scheduled ingestion](../indexer/src/api/local_quote/ingestion.rs) and
  [separate skeleton cache](../indexer/src/api/local_quote/skeletons.rs).
- [Independent references](../indexer/src/api/local_reference_quote.rs),
  [hook pricing](../indexer/src/api/local_hook_pricing.rs) and
  [native wrapper observations](../indexer/src/api/local_wrapper_quote.rs).
- [Client validation and signing anchors](../frontend-dapp/src/utils/localRouteQuote.ts),
  [exact protection constraints](../frontend-dapp/src/utils/slippageConstraint.ts),
  [route client](../frontend-dapp/src/utils/cw20RouteSolveQuote.ts) and
  [socket regression corpus](../indexer/tests/route_local_quotes_1452.rs).

Configuration and reserve publication use separate schedules. A delayed reserve
pass cannot replace a newer configuration epoch; a known wallet block cannot
be erased by unavailable discount data or a failed blacklist refresh. Wrapper
costs participate in gross-input fair deviation and cannot define their own
reference to bypass the 99% guard. Source exceptions and unfinished acceptance
are recorded in [RESULTS.md](../docs/qa/issue-1452/RESULTS.md); never equate a
passing local verifier with complete issue or production acceptance.

Known negative wallet contexts survive recency-census eviction. Each scheduled
pass checks eight rotating recent wallets from a 120-wallet census and eight rotating historical wallets; at most
2,048 contexts are retained. An explicit fresh unblock clears a block even when
fee lookup fails. Capacity overflow must quarantine runtime quotes immediately,
including cached responses, without evicting a block. Authoritative complete
inventory is required for recovery; a skipped verification or partial census
cannot clear this protection. This is not a human approval/reset task.

Community-tax supported scope, exact gross/economic metadata and unsupported
cooldown/AutoLP observations are documented in [follow-up tax checks](../docs/qa/issue-1452/FOLLOWUP-TAX.md).
Named actors require scheduled per-token identity/cap observations; do not introduce
request-triggered discovery or wallet-funded fallback to broaden this scope.

Follow-up local modes and the direct zero-hop signing limitation are documented
in [FOLLOWUP-MODES.md](../docs/qa/issue-1452/FOLLOWUP-MODES.md). Hook inverse must
preserve independent rounding collisions; declared POST plans bind every exact
hop budget/maker cap and cannot be replaced by deeper search. A `wrapper_only`
result with `execution_supported:false` is useful arithmetic, never permission
to send a floorless external wrap/unwrap transaction. Version-1 direct signing
and composed inverse/wallet coverage follow [QC1459](AGENTS_LOCAL_QUOTE_COVERAGE.md)
and its [agent checklist](../docs/qa/issue-1459/README.md).

Incremental configuration, per-pair expiry and epoch-checked authoritative inventory
recovery follow [FOLLOWUP-INGESTION.md](../docs/qa/issue-1452/FOLLOWUP-INGESTION.md).
Publish completed scheduled observations before further work; request traffic must
never advance the slice cursor or reset quarantine.

Provider lifecycle/admission recovery and useful-result latency: preserve
[PR1464](AGENTS_PROVIDER_RECOVERY.md), run `make verify-issue-1464` and follow
[AC1–AC9](../docs/qa/issue-1464/README.md). No availability fix may introduce
request RPC, renew stale observations or weaken reviewed execution/signing guards.
