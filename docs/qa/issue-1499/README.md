# Quote lifecycle verification (#1499)

Issue: [#1499](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1499).
Agent contract: [QL1499-1–QL1499-10](../../../skills/AGENTS_QUOTE_LIFECYCLE.md).
Related fixes: [Trade entry #1497](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1497),
[stable layout #1498](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1498),
[order metrics #1500](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1500).

Two independently established source boundaries explain possible quote loss:
missing/stale HTTP503 previously erased still-unexpired same-intent display;
optional scheduled router configuration failures previously became Boolean false
and then HTTP422 unsupported_semantics. The latter is a classification defect,
not proof that it caused every historical/live reported rejection. Router reads
now retain unavailable versus authoritative incompatible configuration as distinct
states; both refuse execution. Required pair-query provider gas/capacity/transport
failures also become missing prerequisites; deterministic contract-query rejection
remains unsupported. No provider request is added to quote endpoints.

Original observation expiry remains unchanged. The frontend schedules local
arithmetic one second before expiry so a normal replacement response can finish
before display expires. Interval duration is anchored to the response dataUpdatedAt,
not a moving render clock: repeated progress/UI renders must not postpone an
ordinary ten-second refresh until the original twenty-second expiry. Near-expiry requests retain their existing one-second
minimum; failures retain their existing ten-second cadence or typed-error interval
stop/publication recovery. A slow/missing producer can still cause a real pricing
wait; expired output never becomes a current estimate or signing payload.

For each path below, retain first/retest results against stable final source. Run
`make verify-issue-1473`, `make verify-issue-1489`, `make verify-issue-1452`,
`make verify-issue-1483`, `make verify-issue-1480` and relevant provider/feed guards.
Only owned disposable fixtures may be reset. Executed failures remain FAIL;
unavailable acceptance may be explicit SKIP but stays unchecked.

| Check | Expected behavior |
| --- | --- |
| Success → delayed success | Same exact-intent unexpired amount, route, costs and notice stay visible; CTA and direct mutation refuse while fetching. |
| Success → typed missing/stale | Matching original unexpired output stays visible; no false unsupported notice or loading pulse; mutation refuses until a new validated response. |
| Success → transport failure | Matching original unexpired output remains; transport and semantic classes stay distinct; no accelerated retry after expiry. |
| Success → genuine negative | Current unsupported/protection/restriction/client-validation response clears executable display; no waiting relabel. |
| Expiry −1ms/0ms/+1ms | Original snapshot exists only before expiry; both output and payload disappear at the original deadline. |
| Early refresh → crosses expiry | Refresh begins before deadline; slow response still clears original data at expiry and becomes useful only after matching new validation. |
| Unchanged/changed publication | No solves for unchanged generation; bounded changed-generation recovery and same-generation better-result promotion remain intact. |
| Amount/flip/account/protection/manual/target edits | Incompatible output and notices clear immediately; no placeholder or late A→B→A response becomes executable. |
| Receive sizing refresh | Unexpired matching pay estimate stays visible, while fetching/error still blocks signing; expired receive snapshot is removed. |
| Router read success/failure | Compatible router returns positive; failed/timeout read returns missing_observations; validated incompatible configuration returns unsupported_semantics. |
| Consent/signing | Reviewed exact operations stay pinned; stale/placeholder/expired/actor-mismatched/indicative/progress-only/unknown-budget mutation refuses; no automatic resend after unknown inclusion/code11. |

Focused source commands:

```sh
bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/vitest run \
  src/hooks/useQuotePricingDataPending.test.tsx \
  src/hooks/useSubmitAlignedSimQuote.test.ts \
  src/utils/localQuoteFreshness.test.tsx \
  src/pages/SwapPage.test.tsx \
  src/components/trade/__tests__/TradeMarketOrderPanel.submitSnapshot.test.tsx
(cd indexer && cargo test --lib issue1499)
(cd indexer && cargo test --lib issue1473_native_pricing_capability_and_rate_caps_are_distinct_from_ui_protection)
```

Live acceptance requires anonymous desktop and phone unchanged-input samples
beyond both 20s and 180s windows, bidirectional USTC↔CL8Y and CL8Y↔cLUNC plus
arbitrary supported identities, wrap/unwrap/manual/receive sizing and input edits.
Record response reason, original observation/expiry, current input ownership,
DOM amount/notice/geometry transitions and bounded requests. Independently attest
served frontend/root hashes and backend health/revision; measure useful output
from original input, not from an already completed/loading/error state. Keep the
socket ≤500ms and current browser ≤750ms scopes separate. Grafana/Loki aggregate
counts and Prometheus availability are supporting context, not proof of these
criteria or production settlement. Preserve failed/unavailable initial setups
separately from successful later retests, and never close this issue using only
source passes or a single positive quote.

Bounded anonymous final-deployment command, safe request/receipt budgets and
independent API stage checks: [LIVE.md](LIVE.md).

Merged implementation: [PR #1501](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1501).
Exact deployment and subsequent live receipts: [joint release results](../trading-1497-1500/RESULTS.md#merged-release-and-deployment).

Latest producer deployment and retained live failures: [release results](../trading-1497-1500/RESULTS.md#producer-deployment-and-final-acceptance-scope). These scopes do not check incomplete acceptance rows.
