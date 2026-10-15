# Native quote recovery (#1505)

Apply to scheduled wrapper observations, native route availability, request
diagnostics, original display expiry and native recovery measurements. Issue:
[#1505](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1505).
Preserve [QL1499](AGENTS_QUOTE_LIFECYCLE.md),
[PR1464](AGENTS_PROVIDER_RECOVERY.md) and [LQ1452](AGENTS_LOCAL_SWAP_QUOTES.md).

| ID | Invariant |
| --- | --- |
| NR1505-1 | Quote, reverse, progress and refresh requests consume local observations only. Independently scheduled producers own all chain reads; demand cannot accelerate their cadence. |
| NR1505-2 | Independent wrapper reads may overlap through the existing shared two-permit/four-RPS lane. Dependencies remain ordered. Run router configuration concurrently with all remaining observation work after validated mapper configuration, within the existing LCD attempt/total bounds and original 7.9-second publication margin. Required router/tax/cap reads must not acquire an artificial shorter subdeadline; the original whole-poll/publication deadlines still cancel owned work. After validated treasury bindings, run independent tax/cap reads alongside mappings within that same original margin. Attach caps only to mappings validated in the same poll. Optional capability reads retain their bounded two-second phase after critical mapping and output reads complete, with neutral cancellation at that phase deadline. Log router publication-margin timeout as fixed router_config/deadline evidence. Preserve the ten-second cadence, eight-second outer deadline, 7.9-second publication margin and original poll-start freshness. |
| NR1505-3 | Missing, timed-out or malformed required fields refuse execution. Concurrent completion never invents a positive, reuses unknown state or renews original twenty-second market/wrapper expiry. Known pauses remain bounded negative records until a validated positive observation, and return the existing unsupported-semantics rejection rather than a recoverable provider error. Positive views and negative records share the original 32-namespace bound. Known negatives remain authoritative; a positively observed mapper/treasury pause revokes the previous wrapper immediately rather than being treated as transient absence or waiting behind unrelated bindings/router work. |
| NR1505-4 | Native request diagnostics use server-generated correlation identifiers and fixed reason/phase classes. Final503s may emit at most one stale candidate witness for each of eight fixed prerequisite components, with bounded age/expiry scalars; failed alternatives are witnesses, not a sole-cause assertion. Successful responses emit none. Never log query strings, wallet identifiers, bodies, secrets or private provider endpoints. Identifiers belong in response headers/log fields, never metric labels. |
| NR1505-5 | Display and signing payload disappear at the original deadline. A delayed React transition cannot leave an expired executable display waiting for a later scheduler task. No expiry callback initiates network validation or renews metadata. |
| NR1505-6 | Browser measurements use the original input event and browser-observed response body clock. Node/CDP delivery lag cannot manufacture observation freshness or silently erase elapsed time. Retain ambiguous display owners until a distinguishable transition; qualify two-frame useful paint independently of first DOM observation. |
| NR1505-7 | Initialized socket useful results must meet 500ms; original-input useful browser results must meet 750ms. Errors, topology and loading do not count. Every selected sustained native response participates in availability acceptance; a typed503 cannot become PASS through later recovery or fresh-display retention. Preserve failed attempts, sample counts, transport reuse and original expiry through a focused native cohort. Record intentional teardown separately from active failures. |
| NR1505-8 | Preserve provider/work/hop/cache limits, original reference quality, exact economics and fail-closed signing. Document actual deployment and live evidence separately from source passes. Executed failures remain FAIL; unavailable evidence remains explicit and does not establish closure. |
| NR1505-9 | Only the owned pool lane may send one coalesced reference wake per completed cycle. Keep the existing reference timer as fallback, one active mark pass and one independently owned accounting pass, with no queued work or request-triggered wake. Publish the same qualified hub/economic batch atomically with original source timestamps; never renew expiry. Accounting consumes one consistent certified context and cannot delay critical mark publication. All futures remain owned and cancellation-safe. |

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
rescan loop; requests never acquire this behavior. Check the deadline before
rescanning and retain cancellation ownership. No admission or network work may
start at or after the original deadline.

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
