# Native quote recovery (#1505)

Apply to scheduled wrapper observations, native route availability, request
diagnostics, original display expiry and native recovery measurements. Issue:
[#1505](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1505).
Preserve [QL1499](AGENTS_QUOTE_LIFECYCLE.md),
[PR1464](AGENTS_PROVIDER_RECOVERY.md) and [LQ1452](AGENTS_LOCAL_SWAP_QUOTES.md).

| ID | Invariant |
| --- | --- |
| NR1505-1 | Quote, reverse, progress and refresh requests consume local observations only. Independently scheduled producers own all chain reads; demand cannot accelerate their cadence. |
| NR1505-2 | Independent wrapper reads may overlap through the existing shared two-permit/four-RPS lane. Dependencies remain ordered. After validated mapper configuration, admit the two common treasury reads before router configuration. Router work then overlaps remaining settlement work within the existing LCD attempt/total bounds and original 7.9-second publication margin. Required router/tax/cap reads must not acquire an artificial shorter subdeadline; the original whole-poll/publication deadlines still cancel owned work. After validated treasury bindings, run independent tax/cap reads alongside mappings within that same original margin. Attach caps only to mappings validated in the same poll. Optional capability reads retain their bounded two-second phase after critical mapping and output reads complete, with neutral cancellation at that phase deadline. Log router publication-margin timeout as fixed router_config/deadline evidence. Preserve the ten-second cadence, eight-second outer deadline, 7.9-second publication margin and original poll-start freshness. |
| NR1505-3 | Missing, timed-out or malformed required fields refuse execution. Concurrent completion never invents a positive, reuses unknown state or renews original twenty-second market/wrapper expiry. Known pauses remain bounded negative records until a validated positive observation, and return the existing unsupported-semantics rejection rather than a recoverable provider error. Positive views and negative records share the original 32-namespace bound. Known negatives remain authoritative; a positively observed mapper/treasury pause revokes the previous wrapper immediately rather than being treated as transient absence or waiting behind unrelated bindings/router work. |
| NR1505-4 | Native request diagnostics use server-generated correlation identifiers and fixed reason/phase classes. Final503s may emit at most eight total prerequisite candidate witnesses: one first original witness per fixed component, shared between missing and stale kinds. Stale events retain their original bounded age/expiry scalars. Missing events use only fixed stage/component labels and explicitly unknown clocks; never label absence as staleness. Failed alternatives are witnesses, not a sole-cause assertion. Successes and non503 rejections emit neither kind. Never log query strings, wallet identifiers, bodies, secrets or private provider endpoints. Identifiers belong in response headers/log fields, never metric labels. |
| NR1505-5 | Display and signing payload disappear at the original deadline. A delayed React transition cannot leave an expired executable display waiting for a later scheduler task. No expiry callback initiates network validation or renews metadata. |
| NR1505-6 | Browser measurements use the original input event and browser-observed response body clock. Node/CDP delivery lag cannot manufacture observation freshness or silently erase elapsed time. Retain ambiguous display owners until a distinguishable transition; qualify two-frame useful paint independently of first DOM observation. Persist actual initialized API rows and sticky failures before Chrome launch via the separate initialized_api_checkpoint_v1 evidence profile. Record existing-call request/response epochs and canonical calendar/weekday-validated server Date or explicit null without changing scoring, clocks or requests. Window entry is not proof those checks passed. |
| NR1505-7 | Initialized socket useful results must meet 500ms; original-input useful browser results must meet 750ms. Errors, topology and loading do not count. Every selected sustained native response participates in availability acceptance; a typed503 cannot become PASS through later recovery or fresh-display retention. Preserve failed attempts, sample counts, transport reuse and original expiry through a focused native cohort. Record intentional teardown separately from active failures. |
| NR1505-8 | Preserve provider/work/hop/cache limits, original reference quality, exact economics and fail-closed signing. Document actual deployment and live evidence separately from source passes. Executed failures remain FAIL; unavailable evidence remains explicit and does not establish closure. |
| NR1505-9 | Only the owned pool lane may send one coalesced reference wake per completed cycle. Keep the existing reference timer as fallback, one active mark pass and one independently owned accounting pass, with no queued work or request-triggered wake. Publish the same qualified hub/economic batch atomically with original source timestamps; never renew expiry. Accounting consumes one consistent certified context and cannot delay critical mark publication. All futures remain owned and cancellation-safe. |
| NR1505-10 | Scheduled provider continuation may wait only for shared-capacity-denied alternatives not yet actually attempted, inside one original absolute deadline and global attempt allowance. Keep the original cursor and attempted-endpoint identity; never restart a failed endpoint or reset query work. Release all permits before owned waits, retain typed integrity on expiry, and require fresh exact-height/expected-chain validation from any successful alternative. Direct/request behavior remains prompt and unchanged. |
| NR1505-11 | Each scheduled owner propagates its original absolute deadline through an owned LCD view before constructing read futures. Nested views take only the minimum; clones share admission/pacing/namespace and retain cancellation policy. Poll, shorter floor, head, pair and nested snapshot scopes remain distinct. A head scope cannot expire later pair jobs. Never store an expired scoped view as a long-lived client, renew a budget, or accept late identity success after its governing deadline. |
| NR1505-12 | Native dispatch plans both denoms' three mapping/rate/cap slots once after validated bindings and before future construction. Keep one active read per denom while its peer has required work outstanding. Only after the peer finishes may remaining work bypass its busy denom gate, while still acquiring the existing global two-read gate. Own completion tokens from construction through success/error/drop, retire unbound work explicitly, and register notification before checking completion. No-plan callers retain strict isolation. Never add reads, provider capacity, a deadline or freshness allowance. |
| NR1505-13 | Plan router priority once after validated mapper configuration and treasury address, before constructing concurrent futures. Router work owns no global/provider permit while waiting for both common treasury reads to acquire their existing global permits and pass the original deadline guard. First polling or cooperative Pending is not proof of admission. Register notification before checking the predicate; no-plan helpers remain prompt. The original router/poll deadline owns and cancels waiting work. Native validation still requires successful validated treasury configuration and bindings; admission never establishes positive data. Preserve read counts, two permits, provider budgets and original freshness. |
| NR1505-14 | Scheduled wrapper observation starts with the mapper data query's mandatory fresh endpoint attestation, without a redundant standalone identity preflight. Each actual data attempt and alternative still verifies the configured expected chain on that exact endpoint and checks its original caller/attempt deadlines before data transport. Never substitute cached identity metadata, remove unrelated discovery/actor trust gates, or change provider admission, quarantine, typed-negative precedence, namespace, cadence or original poll-start freshness. Healthy identity/provider attempts reduce from fourteen to thirteen without additional attempts; the thirteen-logical-data-read ceiling is unchanged. |

Observer timing follows the accessibility signing contract: validate the unchanged
original `observed_at_ms`/`expires_at_ms` window up to **30 minutes**, while
requiring published `snapshot_age_ms` at quote construction in **0–20,000ms**.
Producer/reference/wrapper admission remains bounded by its original twenty-second
freshness rules. Never apply a new `now - observed <= 20s` restriction to a
reviewed quote, renew its expiry, or treat an indicative protection response as
signable. Indicative responses retain their actual original expiry (at most
thirty minutes), without execution payload fields. See the [observer correction
evidence](../docs/qa/issue-1505/RESULTS.md#observer-signing-window-correction-2026-10-18);
prior failed receipts remain immutable.

Run `make verify-issue-1505` and the applicable lifecycle/provider checks. Follow
the [focused checklist](../docs/qa/issue-1505/README.md) and record actual results
in [RESULTS.md](../docs/qa/issue-1505/RESULTS.md). This issue excludes physical
wallet/device acceptance and repetition of the earlier exhaustive matrix.

Canonical publication preparation follows
[RD1489](AGENTS_ROUTE_DISCOVERY.md): reuse only previously bound immutable view
digests, recalculate policy/reference/final identity and retain all original
expiry and commit guards. Source verification includes fresh-hash equivalence
and the unchanged maximum-publication useful-HTTP profile.

Scheduled ingestion may wait after a full zero-attempt capacity/circuit deferral
only inside its existing total deadline. Preserve typed deferral provenance,
provider backoff and ordinary half-open admission. A real attempt ends that
zero-attempt rescan loop. The narrow NR1505-10 continuation stays inside the same
provider scan and revisits only capacity-denied, unattempted alternatives; it
never restarts the attempted query or its budgets. Requests acquire neither
behavior. Check the deadline before each admission and after pacing; retain
cancellation ownership. No admission or network work may start at or after the
original deadline. Emit at most one fixed continuation witness per logical
query, without paths, endpoints, actors or upstream bodies. Controlled continuation
proof is separate from an unobserved historical provider-admission sequence.

An identity attempt timeout records provider failure before metadata cleanup.
The cleanup shares the original total deadline, including lock contention;
cancellation cannot lose quarantine or double-count a failure. Historical
verified metadata never admits data without a fresh positive identity read.

Each poll owns a fixed set of data-read futures. A local FIFO dispatcher admits
at most two logical LCD reads at once, before polling their transport futures;
queued work must not consume an already-running query's deadline. Dispatch waits
remain inside the original whole-poll/publication margin, check that deadline
before starting a read, and release on cancellation. The shared provider lane,
rate/attempt budgets and original read count remain unchanged. No global queue
or detached work is introduced; required reads queue before later capability
work. Box transport futures before constructing their enclosing async state.

Each known native denom initially owns one poll-local slot shared by its
individual mapping, rate and cap reads. Acquire it before the global two-read
slot; never hold an outer mapping slot and reacquire it inside child reads.
NR1505-12 makes this isolation work conserving: after the peer finishes all three
planned reads, remaining reads may bypass their busy denom slot and use the
unused share through the same global gate. Conditional same-denom concurrency
can reach two; total concurrency remains two. The former strict isolation is
retained while peer work is outstanding and for callers without a validated plan.

Initialize the fixed plan once after validated treasury bindings, before read
futures are constructed. Unbound denoms explicitly retire their three slots.
Nonduplicated completion tokens are owned from future construction and retire
work on success, error or cancellation. A peer notification is registered before
checking completion, avoiding lost wakes; waiters remain poll-owned. Common
prerequisite, tax and router reads use the global gate. Do not add tasks, global
queues, RPCs, provider permits or time. Missing fields still fail closed; no
previous field is spliced into a new observation. This cannot guarantee every
slow provider fits the original deadline.

The existing completion event may expose only native_borrowed_reads in 0..6.
It counts bypassed logical reads admitted through the global gate, not provider
or wire attempts. Export it from the existing bounded sample, with no extra
query. Retain the 300-line and 18-read limits, fixed poll identity and numeric
sample time; never export raw labels or actor/provider values.

Failed-read evidence uses fourteen fixed slots, fixed phases/reasons and
`denom_dispatch`, `global_dispatch` or `lcd_read` stages. Preserve one terminal
record per polled physical read, including cancellation, under its original poll
span. Queue/active/poll elapsed values are nonnegative diagnostics capped at
60,000ms, never new freshness clocks. `lcd_polled` and `active_ms` refer to logical
LCD-future work, including internal admission/identity waits; they do not prove
an HTTP/provider attempt. Aggregate deadline events without a slot are separate.
Never export query strings, addresses, actors, credentials or raw log lines.

Native observer v9 counts every solve/progress call against one 800-call cohort
allowance, including its four initialized API solves. Keep the shared context
guard at 600 and grant only `min(600, global_remaining)`; stop on either context
or global exhaustion and retain incomplete/FAIL. Do not reset between windows,
change application polling, or exclude setup/progress calls. Generic cohorts
remain 600 per shard and 1,800 total. Preserve four sequential 190-second native
windows, the 16-minute bound, original input/expiry clocks, every availability
outcome and the 500/750ms budgets. Versioned resource correction does not turn a
prior unavailable response or incomplete cohort into a pass. See the
[retained v8 failure](../docs/qa/issue-1505/deployment-1516-browser-fail.json).

Window completion comes from natural active-duration completion, never setup or
response-drain elapsed time. Budget exits and setup/active exceptions remain
incomplete; a full-duration window can still be complete and FAIL availability.
Native monitoring has an explicit fixed 18-read profile for bootstrap, counts,
bounded samples, optional requested-ID correlation and optional Prometheus.
Shared monitoring retains its default 16-read profile; exercise actual capped
transport in tests, not only a fake counter. Keep 300 sampled lines and eight
requested IDs as hard bounds, with no extra query or raw-data export.
The monitor compatibility key `reference_committed` matches the actual hub event
`Hub mark cohort committed` / `hub_marks`; source-bind aggregate and context
queries to that producer literal. Hub economic mark publication does not certify
execution-reference coverage. Preserve obsolete-label counts as incorrectly
labeled evidence rather than inferring absent publication; see the
[qualified historical diagnosis](../docs/qa/issue-1505/RESULTS.md#committed-event-monitoring-correction-2026-10-18).

Endpoint data transport must check its original total and intrinsic attempt
boundaries before identity work and after any awaited identity/metadata step.
Tokio's inner-first timeout polling cannot authorize a first data-transport poll
at or after the governing deadline. Preserve the earlier deadline's origin:
own-attempt timeout charges provider failure; a shorter independently scheduled
caller limit retains neutral cancellation for closed providers. Incomplete
half-open probes and generic cancellation still quarantine. Do not turn expiry into a normal completed error that disarms those
health rules. Tests observe transport-future first polling, not just server wire
receipts, and retain prior typed integrity/known-negative callbacks.

Enclosing budgets reach LCD data and identity paths through explicit scoped client
views, not hidden global/task-local state. A new view only tightens the inherited
deadline; it never creates provider capacity or a fresh attempt allowance. Start
each pair's view inside its spawned owner at the original job origin, and keep
head-only scope separate. Nested snapshot helpers scope once before their retry
loop. Optional floors use a tighter view with the same dispatch and poll-start
clock. Late positive identity completion cannot bypass an elapsed caller/attempt
boundary; known typed negatives remain latched and retain precedence.

Positive data and identity completion must remain inside the governing absolute
deadline, including buffered bodies resumed late. Retain known typed validation
failures first, then reject expired positives before health success or shared
height publication. Keep the original distinction between actual observations and
query eligibility: a timely attested head below a pool's requested minimum may
update the normal global lag watermark, but cannot refresh that pool or invent the
requested minimum as an observed height. Preserve the existing minimum-head
regression unchanged and test a subsequent lagging provider against the actual
watermark. Expired buffered heads publish nothing.

Router priority is a poll-local ordering rule, not extra capacity. After mapper
validation, both TreasuryConfig and DenomBindings must obtain the existing global
permits and pass the unchanged read deadline check before the router gate opens.
A semaphore future may return Pending because Tokio's cooperative budget expired
before the request reached its FIFO. Never signal this gate from first polling.
The gate waits outside the physical router read, so its wait owns no provider or
global permit and cannot be reported as LCD activity. The original bounded router
owner cancels it on expiry; known-negative/error paths retain their normal
cancellation and precedence. No-plan helpers have no added wait. Admission is
separate from validation: downstream native work still depends on successful,
validated common observations from the same poll. Retain the real router-first
OLD/FIX availability controls, a cooperative-exhaustion regression and the
original full wrapper suite. These controlled schedules do not identify the
unobserved historical production order.

The mapper query already owns fresh identity verification before any data HTTP
work. NR1505-14 removes only the wrapper producer's preceding serial identity
probe; it does not remove the query's attestation or authorize a cached positive.
Every alternative repeats that verification on its actual endpoint. Known
wrong-chain/malformed evidence, quarantine and existing fallback precedence
remain authoritative; caller/attempt expiry still prevents late data work.
The original configured namespace and poll-start clock are unchanged. A healthy
full poll without additional attempts has thirteen identity/provider attempts and
thirteen logical data reads; an
availability control clipped by the original deadline may have fewer actual
wire reads, and optional floor capability can remain unknown. Preserve those
actual counts and neutral caller cancellation rather than claiming full optional
capabilities. Legacy Chain-slot diagnostics and isolated historical ordering
controls remain readable. Uniform-identity-delay OLD/FIX controls prove the
redundant source work, not the unobserved identity latency of a production poll.
