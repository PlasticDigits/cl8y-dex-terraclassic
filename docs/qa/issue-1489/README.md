# Automatic route discovery verification

Implementation: [PR #1490](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1490).

Issue [#1489](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1489);
follow-up to [#1483](../issue-1483/README.md).
Apply [RD1489-1–RD1489-10](../../../skills/AGENTS_ROUTE_DISCOVERY.md).
Implementation and receipts: [RESULTS.md](RESULTS.md).
Neutral native entry follow-up: [EMPTY-ENTRY.md](EMPTY-ENTRY.md).

## Procedure

1. Read the complete issue and comments with `fj`. Preserve the independently
   reproduced native 10,000 LUNC gross failure; distinguish real same-cohort
   executable three-hop eligibility from sequential individual-leg arithmetic.
2. Run `make verify-issue-1489`, then related `verify-issue-1483`, `1452`, `1473`,
   `1479`, `1480`, `1464`, `1453`, `1391`, `1398`, `1381`, `1390`, `1396`.
   Fix failures, reset only owned affected fixtures, and retest stable source.
   Private QA control credentials create disposable child databases; runtime
   or migration credentials must never be used for integration tests.
3. Cover relevant versus unrelated producer changes, all four occupied slots,
   repeated cache hits, cancellation, original expiry, bounded dense/cyclic
   enumeration and native three-hop promotion. Compare exact gross native,
   matched-credit wrapped and equal-gross wrapped separately. Compare declared
   plans and supported receive with their own objective and constraints.
4. Check actual desktop/phone Swap/Trade adoption, exact progress intent,
   cancellation and reviewed transaction pinning. Confirm one selected-route
   notice and exact >5%, >30%, >=99% protections; diagnostics cannot authorize
   signing or replace unavailable attribution/reference/budget evidence.
5. Execute owned native three-hop and wrapped controls and rejected hop/final
   floors. Independently reconcile reserves, output wallet net and atomic
   rollback. Ante/network fees remain outside economic rollback. Synthetic
   requested gas is separate from #1480 calibration and physical signed fields.
6. Attest the exact backend, frontend and every served root script separately.
   Run bounded live API and actual-browser checks beyond 20s and 180s cycles.
   Preserve original useful socket <=500ms/browser <=750ms clocks and report
   counts, p50/p95/p99/max. Classify bounded Grafana/Loki/Prometheus results.
   Never publish credentials, actors/digests, private endpoints or raw logs.

## Acceptance checklist

Whole criteria stay unchecked until every requirement has evidence. Record
verified subsets and unavailable scope in [RESULTS.md](RESULTS.md).

| Complete | Criterion | Required evidence |
| --- | --- | --- |
| [ ] | AC1 actual failure stage | Immutable exact-amount counterexample and classification of restrictive constraints. |
| [ ] | AC2 bounded promotion | Highest-net evaluated eligible result converges despite unrelated churn and temporary saturation; relevant negatives revoke unsafe work. |
| [ ] | AC3 API/search consistency | Solve/best/automatic and declared POST/receive obey their documented objective; wrapping and truncation represented truthfully. |
| [ ] | AC4 actual clients | Desktop/phone Swap/Trade adopt improvements, exact progress intent, pinned review and factual concise copy; exact controls remain. |
| [ ] | AC5 economic execution | Original raw attribution/context/expiry/funding/floors, zero RPC and owned native-three-hop/wrapped settlement plus atomic rejected-floor rollback. |
| [ ] | AC6 repeated/deployed evidence | Stable boundary/state/resource/abuse first/retest, artifact attestation and sustained initialized live clocks/monitoring. Missing SQL/device/contract evidence explicit. |
| [x] | AC7 documentation | API/search/progress/invariants/checklist/skills/issue/fix and #1483 crosslinks agree. |

## Commands and scope

`make verify-issue-1489` executes meaningful producer/discovery, owned socket,
frontend and probe-boundary checks twice and checks stable source. Named
`VERIFY_ISSUE_1489_SKIP_<GROUP>_REASON` values support concrete unavailable RUST,
DB, FRONTEND, TYPES or FORMAT scope only after feasible checks are attempted.
The public probe uses anonymous read-only quote planning; a POST to route solve
does not sign or broadcast a transaction. Live provenance and funded execution
remain separate receipts.

The deployed tools require the exact merged revision and the independent
all-root attester receipt. Run anonymous API replacement-cycle checks:

```sh
python3 scripts/qa/probe-route-discovery-1489.py --revision <merged-sha> --cycles 3 --output /tmp/discovery-live.json
```

The actual desktop/phone Swap/Trade tool uses the source-bound strict validator
and original-input two-frame clock, verifies all root artifacts before/after,
requires actual progress requests to match solve, and checks exact raw DOM
adoption at 0/25/185 seconds. It is bounded to 600 reads and 240 seconds with
no signing, actors, provider requests, WebSockets or captured screenshots:

```sh
EXPECTED_SOURCE_SHA=<merged-sha> DEX1489_ATTESTATION_FILE=<attester-pass-json> \
DEX1489_BROWSER_OUTPUT=/tmp/discovery-browser.json \
bash scripts/with-node.sh -- node scripts/qa/browser-route-discovery-1489.cjs
```

Monitoring uses scoped credentials injected by openbao-tools and publishes only
fixed low-cardinality aggregates; a missing Prometheus datasource stays SKIP:

```sh
bao-exec --only COOLIFY_URL,COOLIFY_WIRE_TOKEN -- python3 scripts/qa/probe-route-monitoring-1489.py --output /tmp/discovery-monitor.json
```

The owned funded driver extends the audited #1483 atomic signer via
`localterra-route-attribution-1483.py --pool-hops 3`, requires the exact
`cl8y.qa.owner=dex-1489` container label and rejects every chain except LocalTerra.
It creates fresh contracts/pools, independently checks all reserves and delivered
net, and rejects each hop/final floor with atomic economic rollback. Current DEX
artifact/input hashes and exact wrapper source/artifact hashes are required.
Its 12M synthetic requested gas does not certify production calibration.

## Explicit skip policy

**PASS WITH SKIPS** applies only when every executed check passes.
**Executed failures remain FAIL** until fixed and rerun. Unavailable acceptance
is explicitly unchecked. Existing tests, HTTP200, a matching generation or a
manually constrained quote cannot establish automatic convergence, independent
production reserves, physical-device signing or production settlement. Leave
the issue open unless all body/comment requirements pass. Human-only custody
actions belong in concise cl8y-pm inbox cards; engineering stays with agents.
