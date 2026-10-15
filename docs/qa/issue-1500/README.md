# Order slippage verification

Issue [#1500](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1500).
Contract: [OS1500](../../../skills/AGENTS_ORDER_SLIPPAGE.md) and
[API/rounding definition](../../route-solver.md#expected-order-slippage-1500).

Run `make verify-issue-1500`, then `verify-issue-1483`, `1489`, `1473`,
`1480`, `1452` and applicable reference/floor/tax/launch checks. Use independent
arithmetic fixtures and owned disposable integration state. Repeat stable final
source; failures require a fix and only the affected owned infrastructure reset.
Executed failures remain FAIL. Unavailable checks remain explicit SKIP, with
overall PASS WITH SKIPS only when executed checks pass. Source checks do not
establish live deployment or funded settlement. Record receipts in RESULTS.md.

| Complete | Criterion | Required evidence |
| --- | --- | --- |
| [ ] | AC1 | Independent signed amount/route-specific direct/multihop/book/hybrid formula, denominator, residue and rounding fixtures. |
| [ ] | AC2 | Isolated size, fee, wrapper, tax/hook, reference, favorable/mixed effects retain correct signs and separation. |
| [ ] | AC3 | Actual desktop/phone Swap/Trade Slippage/Fee/route row, one DETAILS disclosure and gas once. |
| [ ] | AC4 | Malformed canonical values, numerator/denominator, operations/cohort, expiry/context and same-intent lifecycle attacks fail appropriately. |
| [ ] | AC5 | Original reference quality, delivered-net ranking, 5/30/99 protections, funding/floors and offline signing budgets pass; independent funded net/rollback receipts separate. |
| [x] | AC6 | API/types/consumer/risk wording/docs/agent guidance distinguish all metrics consistently. |

Live acceptance also requires separately attested served frontend/backend/root
and contract artifacts, original useful-output socket <=500ms/browser <=750ms
timing and sustained Grafana/Loki/Prometheus observations. Synthetic source
arithmetic cannot independently attest an oracle or physical wallet signing.

Merged implementation: [PR #1501](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1501).
Exact deployment and subsequent live receipts: [joint release results](../trading-1497-1500/RESULTS.md#merged-release-and-deployment).

Latest producer deployment and retained live failures: [release results](../trading-1497-1500/RESULTS.md#producer-deployment-and-final-acceptance-scope). These scopes do not check incomplete acceptance rows.

AC6 is completed by the merged API/types/consumer/copy/docs and OS1500 guidance. All other rows retain their independent execution and live verification limits.
