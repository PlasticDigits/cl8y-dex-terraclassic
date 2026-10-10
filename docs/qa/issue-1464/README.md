# Issue #1464 — provider recovery and useful live routes

Issue: [#1464](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1464).
Agent invariants: [PR1464-1–PR1464-10](../../../skills/AGENTS_PROVIDER_RECOVERY.md).
Operational steps: [recovery/readiness](../../runbooks/indexer-provider-recovery.md).
Results and remaining scope: [verification.md](verification.md).
Deployment/history acceptance remains in [#1456](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1456).

## Execute and retest

Use an isolated worktree, host Rust and [DB1396 disposable QA credentials](../../../skills/AGENTS_LOCAL_POSTGRES_DEV.md).
Do not give runtime the QA/migrator env, migrate at startup, reset shared volumes
or change migration history. Each integration process creates its own child DB.

```sh
make verify-issue-1464
make verify-issue-1452
make verify-issue-1453
make verify-issue-1394
make verify-issue-1392
make verify-issue-1396
make verify-issue-1391  # reference/quorum propagation when scheduler changes
```

The [focused verifier](../../../scripts/verify-issue-1464.sh) runs provider,
checkpoint, token admission and oracle scheduler regressions twice, disposable socket/book/health
fixtures twice, and evidence-tool/crosslink checks. The [tax recovery fixture](../../../indexer/tests/tax_admission_recovery_1464.rs) checks typed availability, required-policy and registered-pair negatives, manager revocation before a canceled balance read, and recovery. The [production-lane overlap fixture](../../../indexer/tests/book_snapshot_loop.rs) exercises actual pool/book/admission workers under one healthy provider with unchanged shared caps. It records revision, UTC times,
commands and stage outcomes. The existing verifiers establish the broader protocol,
browser/signing, discovery, attestation and real database privilege regressions.
Repeat on stable final source if source changes during a run.

## Agent checklist

Read the issue and all comments with scoped workstation secret injection:
`bao-exec --only FORGEJO_PLASTICDIGTS_PAT -- fj issue view 1464`.
Record PASS/FAIL/SKIP and actual evidence for every row. Check a box only for the
scope proven by its evidence; a synthetic test cannot check deployed acceptance.

| Complete | Criterion | Agent action and required evidence |
|---|---|---|
| [ ] | AC1: four executable directions | Run quote socket fixtures twice. Test LUNC→cUSTC, CL8Y→USTR, USTR→cUSTC and ALPHA→CL8Y at the exact documented ordinary/small raw amounts, each supported direction/mode and 6/18 decimals. Validate positive net, concrete operations, matching input, original generation/expiry, per-hop/final positive floors and honest genuine liquidity/protection rejection. Run the public corpus against the independently confirmed deployed revision. Topology-only 200 is insufficient. |
| [ ] | AC2: transient recovery preserves serving/checkpoint | Inject capacity deferral, circuits open and attempted height/hash/body/transaction errors repeatedly, then recover. Assert API socket stays alive, original checkpoint/hash retained and same block committed once, without duplicate swaps/fees or restart storms. Test under concurrent pool/book/config/reference producers. |
| [ ] | AC3: fatal integrity/cancellation | Inject checkpoint mismatch/reorg, parser/accounting/database permanent failure independently. Assert progress halts with truthful readiness, no successful skip and no availability downgrade. Cancel retries, shutdown and leadership loss; assert tasks/permits release and workers join. |
| [ ] | AC4: bounded independent observation recovery | Run provider/book/admission regressions twice. Inject unrelated fee/bid/ask/token/actor phase failures while a healthy alternative remains. Check original times and generation, finite request/rate/in-flight/half-open counts, no stale relabeling and rebuilt coverage before actual expiry. Compare deployed coverage across multiple freshness cycles; local cadence alone cannot certify it. |
| [ ] | AC5: accurate blockers | Test missing versus stale versus unsupported versus genuine amount/protection failure and final-response admission separately. Inspect fixed internal reason counters and the single appropriate public blocker. Infrastructure loss must not become a size suggestion; diagnostics must omit actor data/private endpoints. |
| [ ] | AC6: strictly <250 ms useful output | Run real initialized socket hits/misses and production-build desktop/phone browser corpus. Count only validated positive executable results. Record sample count, p50/p95/p99/max, first/later result, cold initialization, new/reused connections and regional RTT separately. Every initialized representative sample must be <250 ms. Any measured miss of the threshold remains FAIL until corrected and rerun. |
| [ ] | AC7: zero request RPC | Run counting/throwing-provider fixtures across GET, best, POST, progress, reference, refresh, confirmation/preflight and reviewed-route preparation. Assert exactly zero network calls, including cache miss/outage/deeper job and concurrent traffic. Independently measure scheduled cadence/finite work; do not subtract unexplained background calls from total counters. |
| [ ] | AC8: isolation/signing races | Run existing #1452 frontend/cache/socket suites. Vary amount/direction/mode/actor/tier/spread/maker cap/funding/namespace/generation and same-key bursts. Exercise refresh/reversal/disconnect/confirmation/deeper-result race; unchanged reviewed operations/floors and nonwaivable ≥99% guard. Check canceled jobs coalesce/clean up within bounds. |
| [ ] | AC9: verifier/docs/deployed sustained trial | Run focused and all five prerequisite verifiers, fix executed failures and retest. Review PR1464 and all crosslinks. Read-only deployed corpus must confirm revision, useful coverage and sustained multiple admission/freshness cycles, plus bounded agent-owned read-only outage/recovery evidence. Keep missing #1456 rollout/history scope open. |
| [ ] | Execution parity/security | Provision feasible funded LocalTerra and verify signed quote/settlement floors/taxes; never broadcast mainnet for reproduction. Exercise malformed/oversized/wrong-chain/ignored-height/overflow providers, spoofed identities, dense/dust graphs, actor block eviction and tax-funding tampering. Runtime/QA ACL tests retain least privilege and per-process DB isolation. |

## Bounded public useful-result corpus

The [read-only probe](../../../scripts/qa/probe-issue-1464.py) uses the issue's
four public token identities and exact ordinary/small raw amounts at 5% / 1% / 30%
protection. LUNC requests set pool-only. Default: 24 requests (12 in the new-socket
group and 12 in a connection-reuse group), paced at least 500 ms, each with a hard ten-second
deadline and 2 MiB response bound. `--cycles` is capped at ten (240 total requests).
Actual transport is observed from socket identity before/after each request. The
reuse group's first connection and any automatic reconnect are recorded as new;
a pooled connection object alone is not proof of socket reuse. No redirects/retries,
signing, credentials or actor contexts. Receipt fields retain
only fixed route names, size class, status/usefulness and timing; raw responses and
endpoint URLs are omitted. Supply the exact revision independently read from health.

```sh
python3 scripts/qa/probe-issue-1464.py \
  --revision <40-character-deployed-SHA> --cycles 1 \
  --output /tmp/issue-1464-public-corpus.json
```

The tool rejects nonpositive/missing/mismatched operations/floors, wrong input/direction,
noninteger raw strings, unsupported/blocked execution, floors above net output, absent
bounded generation and expired/future observations. It reports useful-only distributions per transport.
It exits unsuccessfully when any expected quote is unavailable/nonexecutable or
any useful socket sample is ≥250 ms. Even a passing corpus is PASS WITH SKIPS:
it cannot prove cache hit/miss initialization, isolated backend compute, regional RTT,
browser transport, settlement parity, request RPC counts or an outage trial.

## Skip policy and receipt

Complete feasible prerequisites/checks first. Unavailable holds, deployment, devices
or fixtures do not block source implementation review: record exact attempted command,
reason, revision/UTC and missing scope as SKIP, leave that criterion unchecked and
report overall **PASS WITH SKIPS** when all executed checks passed. No new verification
work is assigned to the human by this checklist. Missing evidence never establishes
full deployed trading readiness or closes #1456.

**Executed failures remain FAIL**, including useful-output/latency assertions,
formatting, compilation, accounting and integrations. Fix and successfully rerun;
never relabel a failure as unavailable with a later skip variable. Reason-bearing
focused flags select only unavailable stages: `VERIFY_ISSUE_1464_SKIP_RUST_REASON`,
`VERIFY_ISSUE_1464_SKIP_DB_REASON`, `VERIFY_ISSUE_1464_SKIP_FORMAT_REASON`,
`VERIFY_ISSUE_1464_SKIP_LIVE_REASON`, `VERIFY_ISSUE_1464_SKIP_PARITY_REASON`.
Default live/parity omissions are explicit and keep those criteria unverified.

Record commit/dirty-patch revision, UTC start/end, exact command/workdir, counts and
first/retest outcomes; checkpoint/hash and original observation ages/generations;
bounded fixed reason/cache/job/provider counters; useful latency distributions/maxima;
and separate source, socket, browser, deployed and settlement scope. Receipts omit
reporters, conversations, screenshots, actor identifiers/digests, wallet histories,
raw upstream bodies, private URLs and credentials.
