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
| LQ1452-3 | Search direct and one-intermediate paths before bounded longer search. Rank amount-specific delivered net output subject to the caller's explicit execution protection policy. Legacy requests retain the actual L9 spread constraint; quote_relative_v1 keeps the exact UI net-floor tolerance and independently derived pinned market-spread bound. Contract identity and checked integer economics determine eligibility; symbols, address order, USD marks and raw cross-decimal reserves do not determine preferences. |
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

For #1473 controlled/deployed browser trials only, explicit user direction on
2026-10-11 temporarily sets a named ≤750 ms criterion; see
[TQ1473-10](AGENTS_TRADING_QUOTE_OUTCOMES.md). The separate socket and other
existing-scope 500 ms checks remain unchanged. Historical 500 ms FAIL receipts
remain failures, and new 750 ms trials do not establish the prior 500 ms target.

Provider lifecycle/admission recovery and useful-result latency: preserve
[PR1464](AGENTS_PROVIDER_RECOVERY.md), run `make verify-issue-1464` and follow
[AC1–AC9](../docs/qa/issue-1464/README.md). No availability fix may introduce
request RPC, renew stale observations or weaken reviewed execution/signing guards.

Complete pool and hybrid views retain independent original watermarks, observation
times and expiry in the [local engine](../indexer/src/api/local_quote.rs).
Zero-book and pool-only budgets prefer the eligible complete pool; automatic book
optimization may compare an eligible original hybrid. A declared positive book
budget must use that original compatible hybrid and must fail unavailable when it
expires. Never splice old books into new reserves/fees or wait for request-triggered
refresh. Changing either whole view changes cache/reviewed-generation identity.
Both views share the bounded [fee registry](../indexer/src/indexer/fee_phase.rs),
negative invalidation and original four-second phase/twenty-second TTL. Verify
exact-input/receive caches and declared plans across independent expiry with
[both scheduled publisher controls](../indexer/tests/scenarios/fee_generation_recovery_1464.rs).

Arbitrary pair/amount reason classification, non-executable protection estimates,
intent recovery and identity entry defaults follow [TQ1473](AGENTS_TRADING_QUOTE_OUTCOMES.md)
and the [agent checklist](../docs/qa/issue-1473/README.md).
Missing/stale data waits and original client-cache expiry are documented in the
[follow-up review](../docs/qa/issue-1473/WARMUP.md); neither drives producers nor
relaxes economic reference admission. Run `make verify-issue-1473`.

Generic route cost attribution, exact-amount cache ranking and consistent fair-reference
explanations follow [RC1483](AGENTS_ROUTE_COST_ATTRIBUTION.md) and
[the checklist](../docs/qa/issue-1483/README.md). Run `make verify-issue-1483`;
explanation never changes reference quality, funding, floors or signing admission.

Bounded automatic deeper discovery, producer invalidation and exact progress/client
adoption also follow [RD1489](AGENTS_ROUTE_DISCOVERY.md) and
[the checklist](../docs/qa/issue-1489/README.md). Run `make verify-issue-1489`.
Same-generation higher-net publication must reach active clients through normal
validation while reviewed transactions remain pinned.

Canonical per-view identity reuse follows
[RD1489](AGENTS_ROUTE_DISCOVERY.md): only exact live immutable Arc/address/kind
bindings may reuse initialized digests. Recompute policy/reference/final identity
and retain all original clocks and commit-time authority. Hash reuse is scheduled
CPU preparation, never a request-side freshness or signing permission. Include
`make verify-issue-1505` and `make verify-issue-1489` for this publication path.

For native producer recovery, follow [NR1505](AGENTS_NATIVE_QUOTE_RECOVERY.md):
overlap same-poll tax/cap work with mappings under the original margin, attach
only validated matching mappings and revoke positively observed pauses. Bounded
zero-attempt circuit waiting belongs only to scheduled ingestion; request paths
remain offline and cannot renew wrapper observations.
Acquire each known denom's one-read mapping/rate/cap gate before the poll's
shared two-read gate. Keep acquisition physical-read scoped, symmetric and
cancellation-safe; do not strand another denom behind both slots held by its
sibling. Fixed failure slot/stage timings describe logical LCD work, not wire
attempts, and never authorize missing fields or renew original expiry.


For native router priority, preserve
[NR1505-13](AGENTS_NATIVE_QUOTE_RECOVERY.md): initialize the common treasury
admission plan before concurrent futures. Open the router gate only after both
reads acquire their existing global permits and pass the original deadline
check. Cooperative Pending is not admission. Waiting owns no permit, remains
bounded by the original router/poll deadline, and never substitutes for validated
common results. No-plan/absent-router behavior and all original budgets remain.

For scheduled wrapper identity, preserve
[NR1505-14](AGENTS_NATIVE_QUOTE_RECOVERY.md): the mapper data query freshly
attests its actual endpoint before transport, including every alternative.
Remove only the redundant standalone wrapper preflight. Keep original deadlines,
provider admission/health, typed negative precedence, configured namespace and
all unrelated discovery/actor trust gates. Healthy identity/provider attempts
reduce fourteen to thirteen without additional attempts, while the thirteen logical data-read ceiling remains unchanged.
Uniform-delay source controls do not prove historical production identity time.
