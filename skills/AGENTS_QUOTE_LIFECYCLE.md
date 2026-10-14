# Exact-intent quote lifecycle (#1499)

Apply when changing Swap/Trade display retention, refresh/expiry scheduling,
receive sizing or recoverable quote rejection classification. Issue:
[#1499](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1499).
Read the [verification checklist](../docs/qa/issue-1499/README.md),
[LQ1452](AGENTS_LOCAL_SWAP_QUOTES.md), [TQ1473](AGENTS_TRADING_QUOTE_OUTCOMES.md),
[RD1489](AGENTS_ROUTE_DISCOVERY.md), [OB1480](AGENTS_OFFLINE_SWAP_BUDGET.md)
and [RC1483](AGENTS_ROUTE_COST_ATTRIBUTION.md).

| ID | Invariant |
| --- | --- |
| QL1499-1 | Display retention requires validated data matching the exact current direction, amount/target, actor, protection and manual mode. Draft/debounced mismatch, previous-key placeholder or hybrid mismatch immediately discards incompatible amounts, economics, floors and notices. No amount scaling or A→B→A late-response adoption. |
| QL1499-2 | An unchanged unexpired validated quote remains visible during background refresh, transport failure or typed missing/stale observations. Signing remains blocked by fetching/error state. Display eligibility never grants execution eligibility. |
| QL1499-3 | Genuine current typed negatives revoke display immediately. Unsupported/protection/blacklist/pause/freeze/restriction/client-validation failures cannot be converted into loading to preserve an attractive amount. Missing/stale 503 is a recoverable publication outcome, not a permanent capability assertion. |
| QL1499-4 | Original metadata expires at its original deadline. Remove executable output and signing payload at expiry; show a stable pricing wait. Start ordinary local refresh at most one second before expiry, with at least one second between near-expiry attempts. Anchor interval durations to the stable response dataUpdatedAt so unrelated observer renders cannot continually postpone refresh. Never change original observation/expiry, renew cached data or accelerate the ten-second unknown-failure cadence. |
| QL1499-5 | Receive sizing separates matching unexpired offer display from conservative execution readiness. Refresh cannot blank the matching pay amount or replace reviewed operations; querying/refetching cannot authorize submission. |
| QL1499-6 | Scheduled wrapper router observations use three states: Some(true) establishes compatible configuration, Some(false) establishes unsupported configuration, None means unavailable read/timeout. Missing router observations return missing_observations, not unsupported_semantics. Every unavailable state still refuses execution. |
| QL1499-7 | Recovery watches independently scheduled compatible publication; progress stays advisory. Unchanged publication cannot repeatedly solve. Preserve the 64-generation recovery bound, existing concurrency/work/hop/cache limits and zero request-triggered RPC. |
| QL1499-8 | Preserve original 20-second market/wrapper and 180-second configuration expiry, four-second fee phases, $10/$100 reference floors, gross funding, actual tax/hooks, positive floors and 5/30/99 protections. Reviewed consent remains pinned; unknown inclusion/code 11 never resends automatically. |
| QL1499-9 | Quote layout reserves its notice/details slots through initial, refreshing, waiting and blocked states. Do not fabricate zero output or retain an expired executable estimate for visual stability. Coordinate [#1498](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1498) and metrics [#1500](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1500). |
| QL1499-10 | Verify and retest meaningful hook/component/HTTP clocks and bounded browser observations beyond 20s and 180s. Report frontend/backend/root artifacts separately, original-input timing, request counts and retained failures. Source passes and text-pattern monitoring aggregates cannot close incomplete deployed acceptance. |

Implementation: [display revocation classes](../frontend-dapp/src/utils/indexerErrors.ts),
[pending wait](../frontend-dapp/src/hooks/useQuotePricingDataPending.ts),
[submit-aligned snapshot](../frontend-dapp/src/hooks/useSubmitAlignedSimQuote.ts),
[original expiry/refresh](../frontend-dapp/src/utils/localQuoteFreshness.ts),
[wrapper observations](../indexer/src/api/local_wrapper_quote.rs),
[native quote admission](../indexer/src/api/local_quote.rs).

Original expiry timers are shared by pay and receive sizing through
[useLocalQuoteExpired](../frontend-dapp/src/hooks/useLocalQuoteExpired.ts). Recheck
the same original deadline when a browser timer fires early; cap timeout wait
length to prevent overflow loops, without renewing expiry or starting requests.

Provider loss never proves an unsupported protocol. Required pair smart-query failures
classify as missing prerequisites unless a typed contract-query rejection establishes
unsupported query semantics; gas ceilings, capacity, HTTP/schema transport and
chain-integrity failures remain non-executable without becoming a capability claim. A validated incompatible
router configuration remains a real negative, and successful malformed schema
reads must never manufacture a compatible configuration. Preserve all independent
signing validations rather than using the display helper in mutation authorization.
