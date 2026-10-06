# Agent protocol: bounded greedy reverse support (#1428)


Implementation: [PR #1432](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1432).
Apply to router greedy reverse admission, active-book quote failures, provider
budget claims or rollout. Issue: [#1428](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1428).
Implementation: [router guard](../smartcontracts/contracts/router/src/reverse_query.rs),
[solver policy](../docs/route-solver.md#greedy-reverse-support-1428),
[integrator contract](../docs/integrators.md#greedy-reverse-route-admission-1428),
[agent checklist/evidence](../docs/qa/issue-1428/README.md).
Preserve [RQ1389](AGENTS_REVERSE_QUERY_BUDGET.md),
[FRQ1422](AGENTS_FUNDED_REVERSE_QUERY_BUDGET.md),
[RT1381](AGENTS_ROUTE_LOCAL_TAX.md), [LG1390](AGENTS_ROUTE_LAUNCH_GUARDS.md)
and [NE1398](AGENTS_NET_EXECUTION_FLOORS.md).

1. **GR1428-1 — admission.** Any reverse route containing `greedy` has at most
   three total operations. Four-hop greedy or mixed-mode reverse returns
   `GREEDY_REVERSE_ROUTE_UNSUPPORTED` before external queries. Admission is
   independent of amount, direction, trader, sender, exemptions and book occupancy.
2. **GR1428-2 — compatibility.** Four-hop pool-only reverse and four-hop forward
   simulation/execution retain their limits. Never silently replace rejected
   greedy requests with pool routes, shorten operations, or return executable guesses.
3. **GR1428-3 — sufficiency.** Admitted reverse requests require a fresh complete
   forward validation of the selected gross input against the original net target,
   in the same trader/sender, funding and operation context. Cached probes and
   pool inverse seeds cannot substitute for this validation.
4. **GR1428-4 — economics.** Preserve authorized gross input, original-trader tax
   and exemption decisions, pair credit/hook parity, route-owned residues, positive
   post-hook execution floors and independent market-spread checks. No intermediate
   wallet debit or tax waiver may make an unsupported quote appear feasible.
5. **GR1428-5 — ceilings.** Keep 33 simulations per hop, 96 per request, 32 for
   opaque pair reverse work, 100 maker fills and 500 scans. Only exact reviewed
   pair identities receive cheap pool accounting. Admission and work counters
   are not provider gas guarantees, even for shorter populated books.
6. **GR1428-6 — errors and intent.** Deterministic admission, work-budget,
   hook-inverse and provider-gas errors stay observable. Receive intent survives;
   stale Pay and executable operations clear. No automatic retry, endpoint fanout,
   weaker quote or resend follows a deterministic rejection. DB exact-output
   sizing remains separate and retains its fresh router forward preflight.
7. **GR1428-7 — evidence and versions.** Reproduce integrated router 1.4.0 /
   pair 1.21.0 on an isolated pinned SDK53 node at the existing 15M book-matrix
   limit, preserving raw FAIL and prior successes. Retest router 1.5.0's policy
   and admitted routes from fresh fixtures. Keep actual smart-query outcomes,
   transaction/wrapper gas, simulation counts and production acceptance distinct.
   Recheck funded pools separately at unchanged 3M. Production rollout remains
   #1384/#1386/#1421; local receipts do not establish deployment.
8. **GR1428-8 — agent verification.** Run `make verify-issue-1428` and affected
   existing verifiers. Complete feasible provisioning/checks first and retest
   corrections. Unavailable checks may be **SKIP** with reason, UTC and revision,
   while overall implementation review is **PASS with explicit skips** if every
   executed final check passes. Keep skipped criteria unchecked. Executed failures
   remain **FAIL** until fixed/retested; frozen baseline failures are reproduction
   evidence, never final-artifact acceptance. Unavailable workflow holds do not
   block source review or waive runtime safeguards or credential boundaries.
