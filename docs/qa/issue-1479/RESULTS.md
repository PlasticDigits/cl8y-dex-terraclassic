# Small-market implementation and scoped verification

Issue: [#1479](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1479).
Implementation PR: [#1481](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1481).
Policy: [SM1479](../../../skills/AGENTS_SMALL_MARKET_REFERENCE.md).
Checklist: [README](README.md). Source started at
`2d1627ffa33eb3e08199acfff574a5b63fcfbd39` in an isolated worktree.

The user explicitly authorized the economic total-TVL floor **$100→$10** on
2026-10-12. It now uses `ECONOMIC_USD_TVL_FLOOR`; hub independently priced-depth
admission retains `HUB_USD_TVL_FLOOR=100`. This changes reporting/economic eligibility
under existing provenance rules. It introduces no token exception or new independent
thin-token oracle quality class. Stronger historical/corroborated manipulation
resistance in the original AC3 remains unproved.

The loader binds economic contract and asset ID, canonical source/anchor identity,
fresh two-source quorum, effective threshold and min(original mark, original
anchor time). Native/CW20 aliases cannot contradict asset kind. Policy identity and
original-time exclusions participate in generation identity. A complete scheduled
candidate census can produce fixed 422 `reference_policy_excluded`; uncertain or
expired prerequisites retain 503. Progress distinguishes search completion from
quote eligibility and checks current expiry/floors/configuration. The client avoids
a solve loop for structural blocks and recovers on eligible scheduled publication.

## Passing source subsets

- `make verify-issue-1479`: **PASS WITH SKIPS**, 12 passing groups, zero failures,
  one explicitly omitted complete funded/native/attack/device/live matrix group.
  Both rounds: 10 targeted Rust tests,3 owned disposable SQL tests,16 socket/recovery
  tests and 143 frontend tests. Types, formatting, crosslinks and stable fingerprint pass.
- `make verify-issue-1480`: source **PASS WITH SKIPS**, 348 frontend tests each round;
  actual entry, exact fee, original expiry, recovery and balance regressions pass.
- Shared exact-output/solve/hybrid/tax API suites: 65 tests passed serial on corrected
  complete source-qualified fixtures, preserving original amount/net expectations.
- Generic small-market fixtures use approximately $25.29 liquidity, 180 bps, both
  directions and exact positive 1/5/30% upward-rounded floors. $10 raw boundaries ±1
  and unchanged $100 independent-depth behavior are tested separately.
- Independent review corrected unresolved progress identities, contract/asset
  poisoning, native kind aliases and inherited anchor expiry. Privacy/matrix bounds
  and documented acceptance limits were reviewed separately.

## Original live diagnosis

[Original economics](live-original-economics.json) pin backend
`ea627f7b0e8d5ebc9fde2bb5b243f451cf159f4d`, no floor environment override,
public original reserves, 180 bps and zero hooks. Two configured providers rejected
bounded reads with 403; one succeeded. Effective runtime database credentials were
redacted by Coolify, so original scheduled timestamps/reference rows remain SKIP.

[Independent direct economics](live-direct-economics.json) repeat beyond 20 seconds:
all six KENA↔cLUNC contract simulations match raw pool arithmetic while the old
indexer returns 503 missing observations. This establishes usable pool economics
and a reporting-policy coupling, not complete configuration/provenance or funded
mainnet acceptance. No actors, signatures, broadcasts or production state mutation.

[Grafana/Loki before deployment](monitoring-before.json): fixed 15-minute aggregates,
no raw log lines. Prometheus is explicitly SKIP because no datasource is provisioned.

## Retained failures and corrected scope

Initial fixture runs failed for missing exported QA capability, invalid short token
identity, an invalid one-source `fresh` oracle certificate and incomplete seeded
reference provenance. Corrected fixtures use owned random child databases and
complete canonical sources; extra source-only pools are execution-frozen, leaving
actual test paths/amounts intact. Earlier unstable-source verifier failures remain
FAIL; final source rounds above pass. A pressure/expiry fixture failed before the
stable serial rerun passed without extending any TTL or fee-phase deadline.

No full criterion is claimed merely from these subsets. Live deployment/browser,
full arbitrary-state matrix, independent manipulation evidence, physical wallets,
calibrated native/tax/hook/book variants and original reference DB timestamps
remain separately scoped. Deployment and final acceptance are recorded below
once performed. The issue stays open unless every body/comment criterion passes.

## Additional owned settlement scope

[Mixed-decimal net protection](funded-mixed-net-protection.json):24 direct/multihop
existing-router cases across6/18/9 decimals, both directions,100/1000 offers and
1%/5%/30% exact net floors passed. Independently read reserves match contract
simulation and actual delivered net; rejected floors preserve reserves/LP shares.
This uses12M fixture gas and does not establish calibrated wallet/native/hook gas.

## HTTP cache ownership and retained latency failures

Dynamic quote/progress/receive client requests and API responses now use no-store;
original generation-aware caches continue owning reuse and expiry. Default-cache
controlled trials measured header-to-browser dispatch stalls (3439.6/635.4ms),
with immediate server completion and no renderer long task. Quote-only no-store
comparison first/retest passed the unchanged500ms criterion (142.5/61ms maxima).
A later actual-source trial still measured788.4ms on Swap desktop. Every failed
trial remains FAIL; exact Chromium internals and complete latency acceptance remain
unproved. This cache boundary is a freshness/ownership fix, not a claimed universal
transport-latency repair.

Actual client first/retest checks passed 104 tests across four client/freshness/
recovery suites and TypeScript. Keeping the unchanged controlled harness and
500ms budget, owned browser profile/build scratch under `/dev/shm` passed both
rounds (50.2/54ms maxima,44 quote reads and zero provider requests per round).
Normal temporary-directory first failed at788.4ms before retest passed128.5ms;
that earlier FAIL remains. Environment isolation does not prove deployed latency.


Final independent review identified a reference-cohort race: separately acquired
hub and economic-mark database reads could cross an atomic publication. The
scheduled loader now requires a consistent read-only snapshot, with a deterministic
concurrent anchor/token rescaling regression. Final verification follows that
repair; earlier results and interrupted checks retain their actual source scope.


## Final reviewed source

[Final source receipt](source-final.json) records all four stable-source entrypoints after
both independent review repairs. #1479: 12 passing groups/zero failures/one explicit
acceptance skip (11 Rust,149 frontend,24 socket invocations per round).
#1480: 10 passing groups/zero failures/three scope skips (351 frontend per round).
Canonical #1452 passes all 14 groups (785 library plus benchmark,95 socket
invocations and 479 frontend per round); controlled first/retest maxima 55.9/53.6 ms
meet 500 ms. #1473 passes 17 groups with two acceptance skips (785 library,81 socket
invocations and 437 frontend per round). Its complete 4250-case controlled matrix
has 3948 useful quotes, zero provider calls and useful p 50/p 95/p 99/max
89.7/147.7/166.7/212.6 ms against 750 ms.

These final checks use owned tmpfs browser profile/build scratch and unchanged
measurement clocks/budgets. Earlier normal-host FAILs remain retained. The previous
completed 1452 run belongs to pre-review source;1473 was explicitly interrupted for
the review fixes and has no final verdict. Controlled source success does not
establish live deployed acceptance or physical wallets.


## Publication and deployed checks

[Implementation PR #1481](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1481)
merged to `0395459971312c31f87d502ef7822733a7250238`. Both Coolify apps finished and
report healthy. [Initial attestation](deployment-attestation.json) and
[post-matrix attestation](attestation-final.json) separately pin backend, frontend
deployment and served root script hashes to that revision. Lazy chunks, production
contract binaries and physical signers remain separate scope.

Normal merge returned 405 and ordinary main push was protected. After bounded
inspection of genuine passing/pending checks, the explicitly authorized per-merge
`force_merge` option bound to exact PR head completed without changing protection,
faking status or force-pushing Git. [Publication/CI receipt](publication-ci.json)
retains pending-at-merge; [hosted pipeline 422](https://ci.cl8y.com/repos/7/pipeline/422/1)
subsequently passed every required step. The unpublished local merge was backed up
and equal-tree soft reconciliation preserved all pre-existing dirty files.

[Six original direct KENA offers](live-direct-economics-after.json) now return
validated executable quotes matching independent raw reserves/180-bps arithmetic
and contract simulation exactly in both directions, with positive upward-rounded
5% net floors. The old backend returned 503 for those same offers. No production
signing/broadcast occurred; pool state and backend revision remained unchanged.

[Full API matrix](live-api-full.json) completed 1,680 requests (420 cases, solve/best,
first/retest beyond 181 seconds): 1,116 validated executable, 217 dust, 322 unsupported,
12 impact blockers and 12 transient missing observations. Warm/reused useful
p50/p95/p99/max 288.52/290.259/291.06/291.94 ms meet 500 ms. Overall remains **FAIL**
for one HTTP 200 client-validation rejection, USTR→CL8Y human 1 at 5% in the first
solve. Its original body was not retained and its cause stays **unproved**.
Same-case retest passed. [Exact-input replay](live-client-validation-replay.json)
passed 12 executable validations; its separate cold 883.045 ms result remains
**FAIL** against 500 ms, while warm 11 max 292.380 ms is recorded separately.

[Original unsharded browser run](live-browser-original-incomplete.json) is **FAIL**:
710/840 samples at the unchanged 30-minute cap, plus 1,110.9/1,101.2 ms latency
failures. [Generic bounded shards](live-browser-aggregate.json) completed all 840
unique cases, both orientations/viewports/cycles, with zero failures and 558 useful
quotes. Useful p50/p95/p99/max 376.9/414.4/444.9/467.3 ms meet 750 ms. Each run stays
under 30 minutes/2,200 reads and preserves the 181-second gap and original input-to-
visible-frame clock. Cache classes have 269 observed misses, zero observed hits and
289 unobserved results; this does not establish full hit/miss acceptance.

[Transport replay](live-latency-transport-summary.json) observed six reused HTTP3
requests at 298.357–300.033 ms send→headers, 304.4–312.4 ms fetch→headers and 1.3–2.9 ms
JSON, no long tasks, no writes, and no-store. Original stalls were not reproduced;
no code cause was demonstrated. Its repeated cached-quote DOM timer was invalid
instrumentation and is excluded; original genuine failures remain retained.

[Monitoring](monitoring-after.json) uses fixed 15-minute Loki aggregates only:
indexer 1,050 lines/eight error-pattern/172 capacity-pattern lines; frontend 7,616
lines/113 error-pattern lines. These are operational clues, not quote or accounting
proof. Prometheus is **SKIP**, no datasource provisioned.
[Build classifications](deployment-log-summary.json) retain three missing temporary-
container cleanup matches per app and zero detected compiler-error/fatal/panic
patterns, with generic Error-identifier matches separate. [Narrow runtime aggregate categories](monitoring-error-classes.json) identify eight
backend HTTP503 patterns and zero panic/fatal patterns. Frontend generic matches
remain unclassified by these narrow missing-file/permission/upstream/panic queries;
they are not silently renamed successful checks. No raw log lines, credentials,
actor identifiers or private endpoints are published.

[Original public anchors](live-public-original-anchor-views.json) and
[bounded capability](readonly-wire-db-capability.json) narrow the baseline: both
floor overrides were unset, public quorum has two venues, but private atomic SQL/
economic/configuration rows remain unavailable. The existing broker permits
forwarding only; the legacy API token returned 401. Sequential projections are
not an original atomic cohort or reconstruction of the earlier 00:21 report.

Application source stays identical to deployed `03954599`; follow-up changes are
evidence/docs and QA-only deterministic selection plus bounded failed-response
diagnostics. Shard/parser/coverage/actual-DOM helper tests passed 17 twice; probe
socket/privacy/clock diagnostics passed 16 twice. Original verdicts, timestamps,
500/750 ms limits and read/runtime caps are unchanged. These subsets do not complete
private provenance/history, independent thin-token manipulation, native/tax/hook/
book gas calibration, physical wallets or the full arbitrary-state matrix.
Both issues remain open. Only #1479 documentation AC8 is complete after final links.
