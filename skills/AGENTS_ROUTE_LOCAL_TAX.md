# Agent playbook: route-local tax funding (#1381)

Use when changing token/router funding, trading capability discovery, route quotes,
Swap/Trade/invoice budgets, or migrating community-tax instances. This is the
current rule for funding version **1**; older #607/#615 wallet-funded instructions
apply only to legacy deployments. Issue: [#1381](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1381).

Canonical references: [protocol and migration](../docs/runbooks/route-local-tax-1381.md),
[acceptance and agent verification](../docs/qa/issue-1381/README.md),
[shared protocol](../smartcontracts/packages/dex-common/src/route_tax.rs),
[security invariants](../docs/contracts-security-audit.md),
[solver](../docs/route-solver.md), [frontend](../docs/frontend.md).

## Invariants

1. **RT1381-1 — gross authorization.** The initial offered amount is the entire
   route budget. Supported routing cannot debit an intermediate trader balance or
   allowance. Legacy direct-pair Sell/SendFrom still uses declared input plus tax.
2. **RT1381-2 — exact economic credit.** For gross `G`, sell offer `S` is the
   largest integer satisfying `S + floor(S*bps/10000) <= G`. The pair receives
   exactly `S`, and the CW20 swap hook declares exactly `S`. This replaces H-01's
   blanket ban on router tax sizing; it permits no arbitrary fee-on-transfer credit.
3. **RT1381-3 — one charge per boundary.** Routed buy tax is deducted at the
   authenticated pair→router swap output. Only the resulting net proceeds fund
   the next sell. Final delivery/refunds use authenticated route transfers and
   cannot apply a second buy or incidental transfer tax.
4. **RT1381-4 — owned funds.** Every hop uses only this transaction's input or
   preceding balance delta. Existing router dust and another user's assets cannot
   fund taxes or become outputs. Integer residue belongs to this route's payer.
5. **RT1381-5 — authenticated identity.** Only the configured router's active v1
   context can select route funding. Bind trader/pair/offer/ask/budget and exact
   final/refund transfers. Keep per-token exemptions, launch guards, cooldowns,
   blacklist, pause/freeze, discounts and the swap mutex. Spoofed hook identity
   cannot waive taxes or debit a victim.
6. **RT1381-6 — quote parity.** Forward/reverse/minimum receive use gross→economic
   sell and gross→net buy at every applicable boundary. Hybrid partitions fit the
   funded economic input. Version-1 router output is already net; never subtract
   final buy tax again. Supported taxed intermediates are eligible, ranked by net.
7. **RT1381-7 — independent discovery.** Trading capability is independent of the
   single-code Create/Manage catalog. Attest actual code/hash/router/version;
   cache all candidate asset identities, rates and per-token exemption state.
   Unknown/unsupported tax capability and discovery failures cannot waive tax or
   silently execute wallet-funded hops. Keep catalog provenance strict.
8. **RT1381-8 — frontend persistence.** Create Token feature disablement cannot
   disable trading tax discovery. Failed refetch retains established protection;
   missing required balance/capability blocks submit, including mutation entry.
   Gross typed/Max/percentage/receive/invoice amounts agree with signed input.
9. **RT1381-9 — atomic failure.** Any failed hop, tax sink, deadline, spread or
   minimum return reverts the whole route, tax distribution and route state.
   Recovery distinguishes capability, budget and ordinary balance errors. Never
   advise purchasing intermediate tax reserves; never automatically resend.
10. **RT1381-10 — factual evidence.** Record revision/time, command and outcome.
    Agents may mark overall verification **PASS with explicit skips** when unable
    to run a check, recording its reason and leaving that criterion unverified.
    A failed runnable check is FAIL. Skipping migration/execution does not prove
    deployment or satisfy AC9. Public evidence uses synthetic accounts only.

## Agent procedure

Run `make verify-issue-1381`, which runs tests and retests. Execute available
chain/database/browser checks before considering a skip. The checklist in
[QA](../docs/qa/issue-1381/README.md) maps checks to AC1–AC10. A skip must use its
documented `VERIFY_ISSUE_1381_SKIP_*_REASON`, contain a concrete limitation and
remain visible in the final report. Do not disable hybrid, alter catalog pins,
expand route caps, or waive configured token tax to make a check pass.

For chain work, use a separate worktree and the test-only
[`localterra-route-tax-1381.py`](../scripts/qa/localterra-route-tax-1381.py), which
creates fresh contracts/pools without resetting shared volumes. It rejects every
chain except LocalTerra. For deployment, follow the coordinated migration order
and read-only capability probes in the runbook; never publish secret/key material.
