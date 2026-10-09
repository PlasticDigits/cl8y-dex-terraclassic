# Issue #1453 — reliable cached liquidity ingestion

Issue and comments: [#1453](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1453).
Implementation: [PR #1454](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1454).
Third-party agent contract: [LF1453-1–LF1453-10](../../../skills/AGENTS_LIQUIDITY_FEED.md).
Completed local acceptance and test counts: [verification](verification.md).
Related [Q1380](../../../skills/AGENTS_PROTOCOL_LIQUIDITY_QUALITY.md),
[PD1394](../../../skills/AGENTS_PAIR_DISCOVERY_CACHE.md),
[NET1392](../../../skills/AGENTS_CHAIN_ATTESTATION.md) and
[DB1396](../../../skills/AGENTS_INDEXER_DATABASE_ISOLATION.md) remain applicable.

The reported 2026-10-09 04:00 UTC subtotal lost one supported reserve contribution.
Equal supported membership before/after and later recovery do not establish an
actual withdrawal, its exact pool identity or a reconstructed complete total.
No reporter identities, screenshots, conversations, wallets or private transaction
history belong in this issue's receipts.

## Run and retest

Use an isolated worktree and host Rust. Configure a dedicated QA control database
via [Postgres guidance](../../../skills/AGENTS_LOCAL_POSTGRES_DEV.md); each test
process creates its own disposable child database. Never reset shared volumes,
mutate production for QA or expose private DSNs. Schema migration is a separate
privileged job; runtime remains least privilege.

```bash
make verify-issue-1453
```

The [verifier](../../../scripts/qa/verify-issue-1453.sh) checks changed Rust
formatting, provider/discovery/book/valuation library regressions, isolated reserve,
liquidity, UTC series, overview and discovery integration tests, and existing
frontend retention/chart unit tests. Functional groups run twice; the desktop/phone
mocked outage/recovery browser suite runs once without chain prerequisites. It records
revision and UTC times, documentation/evidence checks and explicit unverified
operational/history scope. Use the commands printed by the verifier to reproduce
individual stages; failures stay failures even if a later retest passes.

## Reviewing agent checklist

Record PASS, FAIL or SKIP for every row. Check a criterion only when the required
scope is established. The verifier is focused source verification; inspect source
and receipts for properties a local test cannot establish.

| Completed | Issue criterion | Agent action and required evidence |
|---|---|---|
| [ ] | AC1: pool success survives fee/book failure | Run `book_snapshot_loop` twice. Inject fee, bid, ask error and timeout separately. Inspect pool reserve raw identity, height/time and retained failed-component timestamp. Ensure hybrid uses its separate committed generation and not fresh-pool/stale-book combinations. |
| [ ] | AC2: steady coverage and bounded failover | Repeat unchanged supported pools with 6/18 decimals. Inject primary outage and healthy secondary/third; assert reserve preservation, bounded attempt count, coverage and recovery time. Include fair pair scheduling with more pairs than the admission burst. Local evidence does not establish production freshness. |
| [ ] | AC3: circuit/rate budget | Run LCD/provider unit tests twice. Exercise 403/429/408/5xx/timeouts, capped Retry-After, concurrent callers, cancellation and perpetual failures. Assert no first-provider hammering, bounded rate/in-flight work, one half-open probe and success recovery. |
| [ ] | AC4: live providers and rejection | Inspect the redacted operational receipt: deployed revision, independently operated configured providers, active provider count, attested chain and height, bounded actual failover/recovery. Test wrong-chain rejection separately. Leave lagging/history rejection unchecked when implementation or original evidence cannot establish it. A single configured provider is not redundant. |
| [ ] | AC5: cached solver/protocol with zero request RPC | Trace overview/series and solver consumers through their selected cache. Count mock network calls on quote/cache miss/outage/deeper search/confirmation. This branch must add no request-triggered refresh. Existing route-solver/exact-output/slippage LCD fallbacks are tracked in [#1452](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1452); leave full zero-RPC acceptance unchecked until that migration is verified. |
| [ ] | AC6: correct totals and latency | Run `protocol_tvl`, `indexer_protocol_liquidity` and `indexer_protocol_liquidity_series` twice. Assert fixed complete totals, 6/18 decimal scaling, supported versus unsupported identities, price-only changes, real decrease/drain controls and terminal UTC stock history. Failed reads retain original age and never create zero. Record ingestion, publication and serving times separately; synthetic elapsed time is not a production SLA. |
| [ ] | AC7: monotonic/atomic generations | Inject delayed older replies, concurrent writers and publication failure. Assert older pool/hybrid writes cannot win, failed publication rolls back, headline/pair stamps/source/grains agree, and hybrids never combine generations. Review migration backfill and explicit pool-versus-hybrid reader choice. |
| [ ] | AC8: historical reconciliation | Audit original 03/04/05 UTC series quality, amounts and supported identity; compare bounded per-pool observations and chain history when available. Preserve original records. Apply only evidence-supported scoped/idempotent repair in a separate job; otherwise retain unknown history and record why no repair was justified. |
| [ ] | AC9: maintained documentation/config/retail | Run the verifier and existing frontend retention/chart tests. Inspect LF1453 links, configuration defaults and source bounds. Desktop/phone refresh/outage/reload keep existing plain amount/date/tooltips; no extra controls or synthetic chart bars. |
| [ ] | Attack: contract and provider boundaries | Run oversized/malformed response, pinned asset identity, checked decimal/overflow, discovery namespace/transient rejection and wrong-chain tests. Contract-local failure cannot quarantine healthy peers or poison negative discovery. Review finite diagnostics and sanitized endpoint logging. |
| [ ] | Database/release | Run DB1396 regressions if schema/startup roles change. Apply migration only with the migrator, require the new schema marker before runtime, and use disposable child databases. Source PASS does not claim production migration/restart. |

## Timing and failure semantics

Pool-only and hybrid book work run in two independently scheduled nonoverlapping
background lanes; the pool lane commits reserves before acquiring the matching fee,
and the book lane consumes those cached observations without another fee query.
Lifetime advisory leadership coalesces each lane across replicas and releases on
cancellation. A 250 ms heartbeat with a 250 ms query deadline cancels outstanding
jobs after a failed leadership connection; detection can take 500 ms. Brief RPC
overlap during failure detection is possible, so this is not distributed exactly-once
delivery. Monotonic publication fences delayed older writes. Pool reads pin
the independently probed cycle height; hybrid fee/bid/ask/order reads pin the cached
pool height. Upstream must attest the exact requested height. Unknown height cannot
produce a new hybrid. Fee observations may cache for 60 seconds only at the same
exact height; advancing heights refresh fee configuration. A cold unknown fee
uses an ineligible sentinel for quoting while the raw pool remains usable for TVL.
`hybrid_pair_reserves` preserves the separately committed compatible reserve/book
generation, including successful empty books. Legacy rows migrate with their
original timestamps, never as newly observed data.

The background cadence defaults to 10 seconds; consumers retain the existing
20-second reserve freshness limit. Pair work remains bounded to four seconds,
height work to four seconds (superseding Q1380’s former two-second head probe) and concurrency to eight pool jobs and four book jobs,
with response/order limits. The two lanes share provider admission, which bounds combined HTTP work.
Background contract phases wait 50 ms on typed capacity deferral inside the
four-second pair deadline, retaining completed phases. Waiters belong to the
scheduled caller futures; no internal queue or request-triggered retry is added.
Each logical query also caps its capacity wait at four seconds for direct helper
callers without an enclosing job deadline.
Actual upstream/contract errors still fail the affected component honestly.
Run `issue_1453_shared_provider_capacity_preserves_all_four_pairs` to verify four
concurrent pairs publish pool/fee/empty-book generations through a sole provider
permit while actual data-handler concurrency remains one.
Configured provider defaults are eight in flight, 64 request tokens/second,
128 burst, at most three endpoint attempts, one-second attempt timeout, four-second
total deadline, 60-second capped backoff and 20-block maximum observed height lag.
Each attempt reserves two tokens for attestation plus data. Clones share client-local
budgets; independently created clients/replicas do not share a distributed limiter.
Response height checks reject observed lag/regression relative to the client's
high-water mark; absence of height evidence is not proof of provider currency.

Provider budgets may defer excess work; fairness and total factory size therefore
matter. Do not promise every pool stays within 20 seconds simply from cadence.
Measure actual deployed coverage and latency before checking AC2/AC4/AC6.
Rate/concurrency admission pressure is a typed deferral. Pair discovery waits 125 ms
and rechecks persistence before retrying; it cannot turn capacity pressure into a
successfully skipped event. Actual upstream failures retain the documented legacy
transient policy in PD1394.

Liquidity publication is a separate cached-input transaction. Existing chart
responses cache for 60 seconds and the frontend polls every 60 seconds. Record
reserve acquisition, publication, API serving and UI polling separately; cache
polling does not renew raw observation time or prove a <250 ms quote budget.
The source slot keeps its last actual attempt, while each grain keeps its terminal
source observation. Historical quality uses observation time, current overview
uses current age. Genuine lower totals and zero still replace retained amounts.

## Skip policy and receipt requirements

Agents may skip unavailable checks while reporting overall **PASS WITH SKIPS**.
Finish runnable checks first, attempt feasible prerequisites, record each missing
item, command, reason, revision and UTC time, and leave the corresponding criterion
unchecked. Unavailable holds/gates or unsupplied operational evidence do not block
source implementation review. An executed assertion, compilation, formatting or
integration failure remains **FAIL**; fix and retest it. A failed run cannot be
retroactively hidden by a skip environment variable.

Reason-bearing skip variables select only unavailable stages:
`VERIFY_ISSUE_1453_SKIP_CARGO_REASON`, `VERIFY_ISSUE_1453_SKIP_POSTGRES_REASON`,
`VERIFY_ISSUE_1453_SKIP_FRONTEND_REASON`, `VERIFY_ISSUE_1453_SKIP_BROWSER_REASON`,
`VERIFY_ISSUE_1453_SKIP_FORMAT_REASON`,
`VERIFY_ISSUE_1453_SKIP_LIVE_REASON`, `VERIFY_ISSUE_1453_SKIP_HISTORY_REASON`.
Record concrete prerequisite evidence, never a generic “agent cannot”. No new
verification work is assigned to the human by this checklist.

Local results establish mocked failure/recovery and disposable persistence scope.
They do not establish active production redundancy, no remaining quote RPC,
production rollout, complete historical reconciliation or a chain-funded LP
control. Keep those limitations explicit even with overall PASS WITH SKIPS.

## Reproduce read-only operational and public-history evidence

The [operational probe](../../../scripts/qa/probe-issue-1453.py) defaults to public
health without credentials. It retains only bounded revision/chain/attestation
fields. Each GET has a hard five-second deadline and a 2 MiB response bound.
Optional Coolify auditing reads only non-preview LCD/expected-chain configuration
and probes at most eight providers (identity plus latest header). At most 18 GETs
execute by default; optional pinned pool probes raise this bound to 26. No
redirects, retries, service writes or raw endpoint URLs enter receipts.
Latest-header success is distinct from smart-query/failover acceptance. Add
`--pool-contract ADDRESS` to the Coolify audit to query that public factory pair
through each configured provider at its observed latest height. Require the exact
`x-cosmos-block-height` response header; missing or mismatched headers are SKIP.
Only the fixed outcome is retained, never pool reserves or response bodies.
Address syntax is bounded lowercase Terra text; the probe does not verify checksum
or independently certify factory membership. One pinned success establishes that
read only, not sustained health or the new implementation deployment.

```bash
python3 scripts/qa/probe-issue-1453.py --output /tmp/issue-1453-public-ops.json
bao-exec --only COOLIFY_WIRE_TOKEN,COOLIFY_URL -- \
  python3 scripts/qa/probe-issue-1453.py --audit-coolify \
  --output /tmp/issue-1453-ops.json
python3 scripts/qa/probe-issue-1380.py --expect-quality \
  --output /tmp/issue-1453-public-history.json
```

Use workstation `bao-exec` from the openbao-tools repository. The token is injected
only into the child process; never print it, dump environment/configuration or
copy credentials into output. `--coolify-app` changes the application ID and
`--health-url` accepts a plain credential-free HTTP(S) URL. The optional audit's
hard timer requires Unix/Linux. Probe receipts remain PARTIAL and enumerate what
point-in-time headers cannot establish. Keep original snapshots before any future
evidence-supported reconciliation; the history probe is a bounded public read,
not historical chain reconstruction.

Published evidence belongs in [operational-receipt.json](operational-receipt.json)
and [historical-reconciliation.json](historical-reconciliation.json). An audit
receipt can report a finding or unavailable scope; its existence is not a pass.
Record configured versus active providers and original published observations;
use bounded fixed fields and omit secrets/credential-bearing URLs, raw upstream
bodies, reporter/wallet identifiers and private transactions. Receipt validation
only verifies shape, not deployment or upstream truth.

```text
Source revision and dirty patch/final revision: <SHA>
UTC start/end; exact command and working directory: <actual values>
Executed: <counts and outcomes, including failures before repair/retest>
Skipped/unverified: <criterion, attempted prerequisite, reason, evidence>
Mock rate/health counts; per-pair height/time/generation: <bounded synthetic data>
Latency: <ingestion / publication / API / UI separately>
Operational/history evidence scope: <what was actually observed>
Overall: <PASS / FAIL / PASS WITH SKIPS>
```
