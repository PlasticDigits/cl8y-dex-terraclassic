# Native quote recovery #1505

[Issue](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1505) ·
[agent invariants](../../../skills/AGENTS_NATIVE_QUOTE_RECOVERY.md) ·
[results](RESULTS.md) ·
[original retained failures](../trading-1497-1500/RESULTS.md#producer-deployment-and-final-acceptance-scope)

Use `make verify-issue-1505`. Source checks run first and retest against unchanged
source. On an executed failure, fix the cause, reset only owned disposable test
infrastructure when relevant, and repeat. Never reset shared or production data.
Unavailable checks can be explicit SKIP with PASS WITH SKIPS. Executed failures
remain FAIL. Skipping a criterion does not establish issue closure.

| Criterion | Evidence required |
| --- | --- |
| AC1 | Trace affected native-input/output paths from bounded scheduled observations through publication, local API and browser. Reproduce and fix a demonstrated availability/recovery cause; retain bounded typed reason/request correlation. |
| AC2 | Correct cohort/clock observer and reproduce or resolve retained display-expiry flags. Test original expiry without stale executable DOM or signing payload. |
| AC3 | Focused original-input initialized socket measurements meet 500ms, with real transport reuse, useful-result validation and error counts. |
| AC4 | Focused native forward/reverse desktop/phone windows cross 20s/180s original expiry and meet 750ms useful-browser budget. Preserve exact input clocks, failures and observer revision. |
| AC5 | Offline requests, original expiry, provider/work limits, actual economics and fail-closed signing remain enforced. |
| AC6 | Attest exact deployed frontend/backend revision, inspect deployment logs and Grafana/Loki; query Prometheus if provisioned. Repeat focused live checks after deployment. |

No broad verification-only sweep, physical device or wallet signing is required
by #1505. Historical failures remain evidence and are not relabeled by new runs.
The issue closes only when its body and comments' actual required checks pass.

After exact deployment attestation, run the focused socket and browser probes:

```sh
python3 scripts/qa/probe-native-recovery-1505.py \
  --revision "$DEPLOYED_SHA" --require-correlation --output "$SOCKET_RECEIPT"
EXPECTED_SOURCE_SHA="$DEPLOYED_SHA" DEX_LIVE_ATTESTATION="$ATTESTATION" \
  DEX_LIVE_ISSUE=1505 DEX_LIVE_SHARD=0 DEX_LIVE_OUTPUT="$BROWSER_RECEIPT" \
  bash scripts/with-node.sh -- node scripts/qa/live-trading-lifecycle-1499.cjs
bao-exec --only COOLIFY_URL,COOLIFY_WIRE_TOKEN -- \
  python3 scripts/qa/probe-native-monitoring-1505.py --output "$MONITOR_RECEIPT"
```

Receipts use private 0600 files. The monitoring helper accepts up to eight
`--request-id` values from a response; it exports fixed enum counts and matching
counts, never raw lines. Monitor collection success is separate from useful-quote
acceptance. Keep failed original runs and exact helper hashes.

Final503 diagnostics retain at most one stale witness for each of eight fixed
components. The monitor exports only allowlisted component names, bounded
age/expiry scalars and explicitly requested server IDs. A failed alternative can
contribute a candidate witness; these events do not alone prove the sole cause.
Successful responses do not emit stale candidate witnesses.

Native browser mode requires useful results during the sustained windows as well
as the initialized probes. Recognized503s remain failures after recovery; ordinary
same-intent display retention does not certify backend availability. Teardown
cancellation is reported separately so closing a completed window cannot create
a false failure or hide an already observed503.
