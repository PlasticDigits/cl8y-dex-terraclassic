# Agent protocol: funded reverse-query provider budget (#1422)

Use for funded router reverse sizing, route-tax previews, receive quote failures
and coordinated rollout. Issue: [#1422](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1422).
Review: [agent checklist and evidence](../docs/qa/issue-1422/README.md).
Architecture: [solver](../docs/route-solver.md#funded-reverse-provider-budget-1422).
Rollout: [runbook](../docs/runbooks/route-local-tax-1381.md).
Retain [RT1381](AGENTS_ROUTE_LOCAL_TAX.md),
[LG1390](AGENTS_ROUTE_LAUNCH_GUARDS.md), [RQ1389](AGENTS_REVERSE_QUERY_BUDGET.md)
and [RTROLL1381](AGENTS_ROUTE_TAX_ROLLOUT.md).

1. **FRQ1422-1 — gross funding.** Reverse returns a sufficient gross input from
   the authorized route budget. Never debit an intermediate wallet, waive tax or
   treat economic pair credit as the amount the payer must authorize.
2. **FRQ1422-2 — scoped reuse.** Cache immutable capability and factory pair-discovery metadata
   only within one query and its original trader/sender/factory/router context.
   Recompute every tax preview and exemption decision with its current amount,
   pair and original trader; never
   cache preview responses or simulations. Token-local exemptions remain distinct.
   Never reuse execution settlement/refund state
   or introduce a cross-request quote cache without complete identity rules.
3. **FRQ1422-3 — fresh sufficiency.** Perform a fresh complete forward validation
   of the selected gross input against the original net target using the same
   trader/sender, operations and funding semantics. Arithmetic seeds and cached
   search outputs cannot replace that validation or weaken signing protection.
4. **FRQ1422-4 — bounded work.** Retain request/hop simulation, route-hop and
   maker/scan limits. Work counts are not provider gas units. Do not increase
   provider query limits or reverse caps merely to make a failing fixture green.
5. **FRQ1422-5 — real envelope.** Reproduce ordinary→ALPHA→BETA→USTR on an
   isolated pinned SDK53 LocalTerra node with actual wasm and unchanged **3,000,000**
   smart-query limit. Record SHA-256, cw2, image ID/version, full query payloads,
   responses/errors and gas receipts. Test all six funded accounting scenarios.
   Synthetic transaction query probes include wrapper/transaction overhead;
   separate actual smart-query success from the measured receipt gas.
6. **FRQ1422-6 — failure handling.** Preserve provider gas failures as observable
   deterministic quote errors. Clear stale executable Pay/operations while retaining
   Receive intent; never auto-retry, fan out endpoints, resend, raise limits or
   silently substitute a weaker trade. Historical network-backed solver DB sizing needs its own fresh router forward
   preflight. The observation-only [#1452/#1459 engine](AGENTS_LOCAL_QUOTE_COVERAGE.md)
   validates its full forward path locally; requests and signing never run router
   preflight or provider queries. Neither local work counts nor local sufficiency
   establish the separate provider gas envelope.
7. **FRQ1422-7 — rollout truth.** The integrated #1422 baseline targets are router
   **1.4.0**, pair **1.21.0**, token **1.3.0**, factory **1.12.0**. Reconcile
   [#1384](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1384) and
   [#1386](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1386) with reviewed
   hashes and fresh read-only quotes at the actual provider's limit before rollout
   acceptance. Local 3M fixtures do not promise universal routes/books/providers
   or prove production deployment. Keep custody and runtime protection intact.
8. **FRQ1422-8 — verification.** Run `make verify-issue-1422`, which tests and
   retests the affected contract/client checks and 3M funded fixture. First attempt
   feasible provisioning. An unavailable check may be **SKIP** with a concrete
   reason, revision and UTC; overall review may be **PASS with explicit skips**
   when all executed checks pass. Leave skipped criteria unverified. An executed
   failure remains **FAIL** until repaired and retested. A frozen expected failure
   is regression-reproduction evidence, never final-artifact acceptance.

Router 1.5.0 adds [bounded greedy reverse admission #1428](AGENTS_GREEDY_REVERSE_SUPPORT.md); use `VERIFY_ISSUE_1422_ROUTER_VERSION=1.5.0` to recheck final artifacts. Existing rollout plans require separate reviewed hashes and production evidence.
