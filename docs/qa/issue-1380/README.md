# Protocol liquidity quality (#1380)

Issue: [#1380](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1380).
Implementation: PR link is recorded in [verification.md](verification.md).
Contract/invariants: [Q1380](../../indexer-invariants.md#protocol-liquidity-quality-1380).
Agent playbook: [AGENTS_PROTOCOL_LIQUIDITY_QUALITY.md](../../../skills/AGENTS_PROTOCOL_LIQUIDITY_QUALITY.md).
Frontend: [liquidity quality](../../frontend.md#protocol-liquidity-quality).
Operational context: [overview rollups](../../runbooks/overview-global-stats-brin.md).

## Contract and terminal semantics

`GET /api/v1/overview.total_liquidity_usd` is now a nullable decimal string.
`liquidity_quality` is `fresh`, `partial`, `stale`, `unavailable`, `verified_empty`, or
`unknown`. Additive `liquidity_as_of`, `liquidity_coverage_identity`,
`liquidity_generation`, `liquidity_reasons`, and `liquidity_max_age_seconds` qualify
it. Fresh means complete **supported catalog** coverage, not every indexed token.
Partial is a priced lower bound, never a comparable full-protocol percentage.
Unavailable/stale/unknown values are null. A zero requires validated fresh reserves
or a recently confirmed empty factory enumeration. Cold inventory without completed
factory enumeration is unknown even if a failed first scan left populated rows.
No last-good value is substituted for withdrawals, swaps, price declines or outages.

Series entries carry `quality`, `as_of`, `coverage_identity`, `generation`, and
`reasons`. Each four-minute UTC slot retains its **last actual attempt**, including
an invalid attempt. Each hour/day/month uses its terminal source sample, never an
earlier healthy sample presented as a close. Degraded numeric subtotals are exposed
with quality for consumers, but the Protocol chart renders them as explained gaps.
Missing buckets are unknown. Historical age is relative to the observation, not
current wall time. Baselines select the nearest terminal sample within ±30 minutes
first, then require fresh/verified-empty quality and identical supported-pool
identity; equal pair counts are insufficient.

The generation transaction takes the shared liquidity advisory lock before reads,
stabilizes input tables, and commits headline, pair stamps, bounded source slot,
all three grains and pruning together. Liquidity as-of retains the oldest used reserve/hub input time (or the
original empty-factory enumeration time). Volume work does not renew it. Cached overview responses must qualify age on
every response; the frontend also expires cached values using the declared TTL.
Hub marks expire at the unchanged reserve TTL. CEX marks expire after ten configured
oracle polling intervals, capped at five minutes. Neither omission is a peg.

## Historical migration and release

`20261004120000_liquidity_quality.sql` preserves legacy numbers and tags legacy
observations `unknown`: they do not contain identity/provenance needed to certify
comparability. Consumers render unknown history as gaps and suppress legacy deltas.
The cited public hours, 2026-10-02 05:00–06:00 UTC and 2026-10-04 02:00–03:00 UTC,
receive an evidence-preserving, idempotent scoped unknown repair. Original rows are
retained in `liquidity_quality_repair_evidence`; no current TVL is copied into history.
Derived daily/monthly values are invalidated only if the repaired source was their
terminal observation. A later valid recovery still defines the close. Run the
isolated migration/repair regression before release and inspect the evidence rows.
This is a quality migration, not reconstructed proof of historical chain balances.

Deploy the nullable API contract and frontend together. SQLx applies the new
migration at indexer startup. Rebuild/restart the indexer and frontend, then collect
read-only before/after public evidence with the probe below. No contract migration
or chain signing is required for this implementation. Preserve the evidence table
when rolling back; old indexer code expects non-null totals and cannot safely run
against this schema without a reviewed rollback. Prefer rolling forward.

## Agent review and verification checklist

1. Record the branch SHA, use Node 24, and configure an **isolated** Postgres database
   in `indexer/.env` (the integration helper loads that file with override). Provision
   `make setup-indexer-postgres` if necessary; never run Cargo in a root bind mount.
2. Run `make verify-issue-1380`. It runs valuation, bounded refresh and Postgres
   failure/recovery tests and frontend quality tests twice, plus types, lint, docs
   links and browser checks. Fix and retest every executed failure.
3. Run `make verify-issue-569`, `make verify-issue-689`, `make verify-issue-655`,
   `make verify-issue-664`, and `make verify-issue-1263`. Preserve their stdout and
   statuses. Review the nullable total and metadata in generated OpenAPI.
4. Review the assertions below and migration evidence. Confirm source snapshots,
   all grains, pool stamp generations, cache expiry and comparable baselines, and
   use `EXPLAIN` on overview/grain queries to verify bounded materialized reads.
5. Attempt `make has-localterra`; provision `make setup-cloud-localterra` if needed.
   With isolated test-funded pools, compare unchanged reserves through an LCD outage
   and recovery, then perform a real local LP withdrawal control. Record chain and
   public pool provenance. After deployment, collect public evidence:
   `python3 scripts/qa/probe-issue-1380.py --output /tmp/issue-1380-public.json`.
   Never publish wallets, credentials, private conversation or production DB URLs.

An agent may mark **overall PASS with explicit skips** if a check is unavailable
and every completed check passed. Record the exact item, attempted prerequisite,
and reason as **SKIP**; leave its criterion unchecked. A skip is never evidence of
successful production deployment or historical reconstruction. An executed failed
assertion remains **FAIL**. Unavailable holds/gates do not block implementation
review under this task's explicit instruction. No verification is assigned to the
human. Verifier skip variables require reason strings: `VERIFY_ISSUE_1380_SKIP_`
`INDEXER_REASON`, `FRONTEND_REASON`, `BROWSER_REASON`, `REGRESSIONS_REASON`,
`CHAIN_REASON`, or `LIVE_REASON`; they describe unavailable scope rather than hide
failing checks.

- [ ] **AC1:** Run fixed-clock valuation and Postgres valid→partial→unavailable→recovery
  tests. No outage creates a fresh zero, -100% or complete zero chart point.
- [ ] **AC2:** Remove a material supported pool's input, delete its reserve row,
  and include unsupported pairs. Inspect reason counts and expected membership.
- [ ] **AC3:** Test known drained pools/confirmed empty factory versus cold unknown
  inventory, withdrawals and price declines. No smoothing or last-good substitution.
- [ ] **AC4:** Interleave both refresh callers, inject failure during publication,
  retry and reject an older generation; inspect coherence and independent cache age.
- [ ] **AC5:** End UTC hour/day/month with an outage after a good sample; qualify
  24h/30d baselines and equal-count membership changes. Run idempotent scoped repair.
- [ ] **AC6:** Run RTL and desktop/mobile browser tests for all qualities, legacy
  payloads, tap/keyboard tooltip, unknown vs known zero and unchanged Volume/Fees.
- [ ] **Abuse:** Verify pinned identity despite spoofed metadata, future/missing
  timestamps, malformed/negative/overflow reserves, bounded slow book jobs,
  failed height probe and fixed allowlisted HTTP grains/limits. Never inject SQL
  or perform production repair via a public mutation endpoint.
- [ ] **Live:** Record deployed revision, before/after JSON, public reserve/price
  provenance and repaired source/grain evidence. Local PASS does not check this box.

Completed assertions and remaining scope are in [verification.md](verification.md).

An empty-factory observation currently comes from complete discovery at startup.
Its five-minute proof expires; an idle empty deployment then reports unavailable
until discovery is rerun. This conservative limit prevents an old inventory result
from asserting a permanently fresh empty protocol; no periodic factory probe is
added by this fix. Existing nonempty pool observations remain governed by reserve TTL.

Freshness also preserves the oldest **used** hub-mark observation, which has the
same cutoff as reserves. CEX eligibility reserves one reserve-TTL of its lifetime
before the maximum oracle age: a nearly expired quote cannot outlive the resulting
cached value. Valued pairs are quantized to 18 decimal USD places before stamping
and summing; positive sub-unit underflows remain unknown rather than rounding to
known zero. A one-sided catalog estimate requires both reserves positive, unless
both are genuinely drained.

The follow-on comments-only migration `20261004120100` replaces old schema comments
that called omitted inputs zero or treated NULL solely as a missing bucket.

Unqualified legacy liquidity **history** is always unknown, including positive
low values. It is drawn as a gap until the API supplies qualified quality; this
prevents an old payload from reintroducing a false collapse during rollout.
[Public health revision](health-before.json), [materialized query plans](query-plans.txt)
and [read-only LocalTerra recorded inputs](localterra-reserve-fixture.json) provide
separate provenance. Recorded-input replay does not claim live LP execution.

The aggregate verifier defaults existing browser regressions to two workers via
`PLAYWRIGHT_WORKERS` to stay within shared-host/API budgets; set it explicitly when
needed. Network-backed smoke tests wait for the relevant stat, not just the page
heading. Browser report output should use distinct directories for parallel agents.

A valid nonfuture completed-enumeration record is required for nonempty inventory
as well: fresh reserve rows from an incomplete first scan cannot manufacture
complete TVL. Nonempty inventory membership persists as the indexed factory set;
reserve/quote freshness remains separately bounded.

An oversized or malformed contract response is a local query rejection, not an
LCD-wide health failure: it cannot cool down healthy pair queries on the same
endpoint. Transport failures still use the existing bounded failover/cooldown.
The LCD regression tests cover an oversized 200/500 response followed immediately
by a healthy pair response on the same client.
