# Native quote recovery (#1505)

Apply to scheduled wrapper observations, native route availability, request
diagnostics, original display expiry and native recovery measurements. Issue:
[#1505](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1505).
Preserve [QL1499](AGENTS_QUOTE_LIFECYCLE.md),
[PR1464](AGENTS_PROVIDER_RECOVERY.md) and [LQ1452](AGENTS_LOCAL_SWAP_QUOTES.md).

| ID | Invariant |
| --- | --- |
| NR1505-1 | Quote, reverse, progress and refresh requests consume local observations only. Independently scheduled producers own all chain reads; demand cannot accelerate their cadence. |
| NR1505-2 | Independent wrapper reads may overlap through the existing shared two-permit/four-RPS lane. Dependencies remain ordered. Start router configuration beside treasury/bindings with its own strict two-second deadline; tax/caps and mapper/treasury capabilities retain their separate two-second phases. Log router timeout as fixed router_config/deadline evidence. Preserve the eight-second poll, 7.9-second publication margin and original poll-start freshness. |
| NR1505-3 | Missing, timed-out or malformed required fields refuse execution. Concurrent completion never invents a positive, reuses unknown state or renews original twenty-second market/wrapper expiry. Known negatives remain authoritative. |
| NR1505-4 | Native request diagnostics use server-generated correlation identifiers and fixed reason/phase classes. Never log query strings, wallet identifiers, bodies, secrets or private provider endpoints. Identifiers belong in response headers/log fields, never metric labels. |
| NR1505-5 | Display and signing payload disappear at the original deadline. A delayed React transition cannot leave an expired executable display waiting for a later scheduler task. No expiry callback initiates network validation or renews metadata. |
| NR1505-6 | Browser measurements use the original input event and browser-observed response body clock. Node/CDP delivery lag cannot manufacture observation freshness or silently erase elapsed time. Retain ambiguous display owners until a distinguishable transition; qualify two-frame useful paint independently of first DOM observation. |
| NR1505-7 | Initialized socket useful results must meet 500ms; original-input useful browser results must meet 750ms. Errors, topology and loading do not count. Preserve failed attempts, sample counts, transport reuse and original expiry through a focused sustained native cohort. |
| NR1505-8 | Preserve provider/work/hop/cache limits, original reference quality, exact economics and fail-closed signing. Document actual deployment and live evidence separately from source passes. Executed failures remain FAIL; unavailable evidence remains explicit and does not establish closure. |

Run `make verify-issue-1505` and the applicable lifecycle/provider checks. Follow
the [focused checklist](../docs/qa/issue-1505/README.md) and record actual results
in [RESULTS.md](../docs/qa/issue-1505/RESULTS.md). This issue excludes physical
wallet/device acceptance and repetition of the earlier exhaustive matrix.
