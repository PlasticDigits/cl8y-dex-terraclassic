# Small-market reference and readiness verification

Issue [#1479](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1479).
Implementation PR: recorded in [results](RESULTS.md).
Read [SM1479-1–SM1479-10](../../../skills/AGENTS_SMALL_MARKET_REFERENCE.md),
[TQ1473](../../../skills/AGENTS_TRADING_QUOTE_OUTCOMES.md),
[oracle safety](../../../skills/AGENTS_ORACLE_SAFETY.md),
[API contract](../../route-solver.md#small-market-reference-eligibility-1479)
and [indexer invariants](../../indexer-invariants.md#small-market-references-1479).

The user authorized reducing the economic **total-TVL** floor from $100 to $10
on 2026-10-12. The hub **independently priced leg depth** floor remains $100.
These are separate policies, with separate environment settings. A $25 pool may
supply an economic mark through the existing qualified anchor policy; it does not
thereby become an independent oracle or waive amount/protection/freshness guards.

## Agent procedure

1. Use an isolated worktree and record revision, dirty source fingerprint and UTC.
   Fetch the issue body and all comments through openbao-tools `bao-exec` and `fj`.
2. Run `make verify-issue-1479`, then relevant `1473`, `1452`, `1464`, `1453`,
   `1391`, `1398`, `1381`, `1390`, `1394`, `1396` checks. Provision only owned
   disposable database/chain fixtures. Never reset shared or production state.
3. Verify below/at/above $10 total TVL separately from $100 anchor depth, both
   direct directions, raw amount/dust/reserve boundaries and unsupported cases.
   Compare meaningful checked net amounts to independently derived pool economics.
4. On failure, fix the cause, recreate only owned fixture state and repeat the
   relevant first/retest suites against stable final source. Preserve earlier FAIL.
5. Attest backend revision, frontend deployment and served artifacts separately.
   Repeat direct/routed/native live tests beyond 20-second and 180-second cycles.
   Loading and successful health responses do not count as useful quotes.
6. Read bounded Grafana/Loki aggregates with the monitoring command below. A missing
   Prometheus datasource is explicit SKIP, not a successful metrics check.

## Full issue acceptance

A whole criterion is complete only when all body/comment requirements pass.
Detailed passing subsets, skipped scope and failures belong in [results](RESULTS.md).

| Complete | Criterion | Required evidence |
| --- | --- | --- |
| [ ] | AC1 proven cause | Effective live floors, compatible original reserves/fees/configuration/anchor/reference rows; classify both directions independently. |
| [ ] | AC2 generic small pricing | Checked amount-specific quotes for valid direct/routed/native small fixtures and precise dust/liquidity/protection/unsupported outcomes; no token exception. |
| [ ] | AC3 reference integrity | Independent provenance, acyclic anchoring, quorum/freshness and manipulation tests; a lowered threshold alone does not satisfy these requirements. |
| [ ] | AC4 lifecycle/readiness | Missing→eligible recovery of the same intent, expiry/policy revocation, distinct search/quote eligibility and no deterministic solve loop. |
| [ ] | AC5 exact execution | Owned funded direct/multihop/native settlement at positive 1/5/30% net floors, rejected-floor reserve/LP rollback and non-signability of stale/indicative output. |
| [ ] | AC6 full regression | Complete specified markets/amounts/modes/state/context/UI matrix first/repeat with meaningful raw expected values and retained failed inputs. |
| [ ] | AC7 deployed acceptance | Independently pinned backend/frontend/artifacts, desktop/phone cycles >20s/>180s, useful hit/miss p50/p95/p99/max ≤750ms browser and separate 500ms socket scope. |
| [ ] | AC8 documentation | Policy/floors, original-time readiness, API/client semantics, issue/PR/skills crosslinks and truthful checklist. |

## Monitoring

```sh
bao-exec --only COOLIFY_WIRE_TOKEN,COOLIFY_URL -- \
  python3 scripts/qa/probe-swap-monitoring-1479-1480.py \
  --output /tmp/dex-swap-monitoring.json
```

This reads Grafana at logs.cl8y.com and only fixed 15-minute Loki aggregate queries.
No log lines, private endpoints, actor identifiers or credentials enter receipts.
Patterns are operational clues, not quote-admission or economic proof.

## Explicit skip policy

Complete runnable checks first. Unavailable capabilities may be explicit SKIP
with concrete attempted command and missing prerequisite. Overall **PASS WITH SKIPS**
is permitted only when every executed check passes. Executed failures remain FAIL.
Skipped acceptance stays unchecked; source tests do not establish deployed or
physical-wallet acceptance. Do not close the issue until all body/comment criteria
pass. Missing independent history is engineering/evidence work, not an operator
approval hold and not permission to invent reference independence.

## Shared deployed matrix

`docs/qa/issue-1479/matrix.json` supplies 420 bounded cases: KENA/cLUNC direct,
CL8Y/KENA, native USTC/LUNC/KENA, and each reported USTR input family, both
orientations, one raw unit, nine human offers and 1/5/30% protection. These identities
select test cases only and never admit runtime assets.

Use `scripts/qa/probe-issue-1473.py --matrix 1479` with a separately attested exact
backend revision. The existing deployed browser
helper accepts `DEX1473_BROWSER_MATRIX=1479`; every existing artifact/revision,
2,200-quote/30-minute and read-only guard remains. Default 1473 corpus is unchanged.
All blocked results remain classified blockers rather than executable successes.
