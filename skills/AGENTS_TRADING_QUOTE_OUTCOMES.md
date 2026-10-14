# Trading quote outcomes and entry defaults (#1473)

Apply to arbitrary pair/amount scoring, quote error schemas, price-only display,
intent recovery and trading entry defaults. Issue:
[#1473](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1473).
Implementation: [PR #1474](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1474); pricing-data and client-expiry follow-up: [PR #1475](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1475); record its exact revision in
[verification results](../docs/qa/issue-1473/RESULTS.md).
Run `make verify-issue-1473` and follow the
[agent checklist](../docs/qa/issue-1473/README.md).

| Invariant | Required behavior |
| --- | --- |
| TQ1473-1 | All supported execution identities and exact entered raw amounts use the same checked economics. The seven-pair corpus is regression coverage, never an admission whitelist. No address/symbol-specific scoring fix or scaled cached price. |
| TQ1473-2 | Preserve bounded typed reasons for missing/stale observations, unsupported semantics, dust, liquidity, accounting, protection, trading restrictions and disconnected routes. An unrelated broken alternative cannot erase a fully priced protection result. Search eligible alternatives within existing caps. |
| TQ1473-3 | Genuine price protection may carry an amount-specific net `indicative_amount_out` on HTTP 422 with `execution_supported:false`. No operations, floors or execution metadata. Missing pricing, accounting, ≥99% fair deviation and unsupported models cannot manufacture indicative output. |
| TQ1473-4 | Main receive output must be executable under the exact selected UI protection. Retail shows one quote and no indicative pricing panel. Original observation expiry also governs client cache freshness, display and signing snapshots; cached data cannot remain fresh for ten seconds when its original expiry is earlier. Price-only API values stay outside executable quote state. HTTP 200 client validation remains a separate failure. Never downgrade HTTP 422 to fallback success. |
| TQ1473-5 | Error bodies are bounded to 4 KiB; frontend allowlists fixed public reason/status pairs and bounded provenance. No server prose, private fields, HTML or upstream status text reaches retail copy/evidence. |
| TQ1473-6 | Missing/stale observation outcomes show an indeterminate price-loading bar, not an error or a false completed percentage/ETA. Intent changes clear obsolete waits/estimates/blockers. Watch local publication even when an initial missing-data error has no generation; re-quote on newly observed generations, suppressing repeat attempts through a bounded 64-generation recent history. Unchanged publication never starts a solve; intent changes reset history. Deterministic 422 remains a truthful blocker. Expiry never accelerates failed-refresh retries: preserve the forward ten-second unknown-error cadence and receive all-error interval stop. Progress observes local state and never drives ingestion; a search-idle label does not establish pricing readiness. |
| TQ1473-7 | The Swap entry default alone is native `uusd` → configured exact CL8Y execution identity, independent of catalog ordering, symbols or liquidity ranks. Missing/ambiguous identities leave selection unavailable deterministically. Valid explicit links and deliberate choices take precedence. |
| TQ1473-8 | Bare `/trade` remains advanced trading and selects a catalog-ranked eligible genuine factory market, or retains its selector when none exists ([#1497](AGENTS_TRADE_ENTRY.md), superseding the earlier bare-entry redirect permission). Only validated explicit native query funding with no genuine factory match may use Swap compatibility. Explicit factory/chart/limit identities remain genuine; no fabricated native factory pair or cUSTC relabeling. Bid/ask and actual funding must match displayed direction. |
| TQ1473-9 | Preserve zero request RPC, independently scheduled producers, original four-second fee phase, twenty-second observations, 180-second configuration expiry and all work/cache/concurrency limits. Retain tax/gross-debit, positive floors, actor/launch guards, original generation and nonwaivable protection. |
| TQ1473-10 | Test first/retest on stable source in a worktree and owned disposable DB/chain fixtures. Measure initialized useful browser output from original input event at ≤750 ms under the explicit temporary user override dated 2026-10-11, excluding errors/loading. Name the budget in each fresh receipt; retain historical 500 ms FAIL verdicts and distinguish prior 500 ms verdicts from current AC7. Report exact revision, UTC, counts and p50/p95/p99/max; separately attest deployed backend and frontend artifacts. |

Complete feasible verification first. Agents may record unavailable checks as
SKIP and the overall tested scope as **PASS WITH SKIPS** when every executed
check passes. **Executed failures remain FAIL** until fixed and rerun. Skipped
acceptance remains unchecked and the issue stays open if any required acceptance
is incomplete. Unavailable workflow holds do not block a reviewable fix; runtime
safety controls remain enforced.

Implementation map: [typed local scoring](../indexer/src/api/local_quote.rs),
[public rejection aggregation](../indexer/src/api/local_quote/diagnostics.rs),
[quote-relative integer protection](../indexer/src/api/local_quote/protection.rs),
[reason boundary](../frontend-dapp/src/utils/routeQuoteErrors.ts),
[local signing validation](../frontend-dapp/src/utils/localRouteQuote.ts),
[original client expiry](../frontend-dapp/src/utils/localQuoteFreshness.ts),
[loading/recovery review](../docs/qa/issue-1473/WARMUP.md),
[entry identities](../frontend-dapp/src/utils/pairCatalogRank.ts),
[Swap](../frontend-dapp/src/pages/SwapPage.tsx),
[Trade](../frontend-dapp/src/pages/TradePage.tsx).

For repeatable read-only deployed acceptance, use the
[deployment attester](../scripts/qa/attest-trading-quotes-1473.py) and
[live browser matrix](../scripts/qa/browser-live-trading-quotes-1473.cjs) through
the [receipt commands and limits](../docs/qa/issue-1473/README.md#deployed-artifact-and-browser-receipts).
The exact merged frontend/backend revision and served root hashes are prerequisites;
incomplete matrices and useful output above the temporary 750 ms browser budget
remain failures. The separate socket budget stays 500 ms; this user override
changes neither quote protection percentages nor economic/expiry/work limits.
Historical failures remain recorded, and passing new 750 ms trials cannot prove
the prior 500 ms criterion. Public receipts
omit credentials, actors/context digests, private endpoints and workstation details.

The QA setup correction is [PR #1476](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1476).
Prepare browser viewport state before the original-input clock: close Settings
after choosing protection and verify Pay/Receive both intersect the viewport.
Keep strict visibility and two-frame checks during measurement; never scroll or
reset the clock to rescue an already measured failure. Record the deployed app
revision separately from a changed QA tool; a bounded reproduction does not
replace the failed full matrix. Follow [the setup/reproduction instructions](../docs/qa/issue-1473/WARMUP.md#live-measurement-viewport-setup).

Preserve [LQ1452](AGENTS_LOCAL_SWAP_QUOTES.md),
[PR1464](AGENTS_PROVIDER_RECOVERY.md), [LF1453](AGENTS_LIQUIDITY_FEED.md),
[QC1459](AGENTS_LOCAL_QUOTE_COVERAGE.md), [NE1398](AGENTS_NET_EXECUTION_FLOORS.md),
[RT1381](AGENTS_ROUTE_LOCAL_TAX.md), [LG1390](AGENTS_ROUTE_LAUNCH_GUARDS.md),
[UF1399](AGENTS_UNFILLABLE_LIMIT_ORDERS.md),
[FRQ1422](AGENTS_FUNDED_REVERSE_QUERY_BUDGET.md),
[PD1394](AGENTS_PAIR_DISCOVERY_CACHE.md), [DB1396](AGENTS_INDEXER_DATABASE_ISOLATION.md).

## Quote-relative policy and legacy compatibility

Retail explicitly opts into `protection_policy=quote_relative_v1`. UI `max_spread`
remains the exact quoted-net tolerance; positive hop/final floors round **up**.
Pinned checked L9 economics derive `execution_market_spread` independently from
known amount impact and the chosen tolerance. Client validation must recompute
the exact 18-decimal ceiling and bind policy, amount, actor, mode, original expiry
and operations before signing. Keep unsupported/unknown policies non-executable,
≥99% fair-deviation and 30% Expert controls intact. Requests without the policy
retain historical L9 admission. Read [the policy formula and compatibility contract](../docs/route-solver.md#quote-relative-execution-protection);
verify [actual existing-router delivery and rollback](../scripts/qa/localterra-quote-protection-1473.py).

Official opt-in helpers require the returned `quote_relative_v1` policy; missing
policy cannot silently downgrade to a legacy floor. Any supplied execution-support
flag must be Boolean and agree across payload/metadata. Route and wrapper
validation plus mutation guards require a safe, nonfuture, unexpired original
observation window of at most 20,000 ms; extending expiry cannot revive a stale
quote. Explicit direct legacy validator tests remain a separate compatibility scope.

Generic route cost attribution, exact-amount cache ranking and consistent fair-reference
explanations follow [RC1483](AGENTS_ROUTE_COST_ATTRIBUTION.md) and
[the checklist](../docs/qa/issue-1483/README.md). Run `make verify-issue-1483`;
explanation never changes reference quality, funding, floors or signing admission.
