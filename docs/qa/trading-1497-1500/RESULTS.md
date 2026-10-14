# Trading entry, layout, lifecycle and order metrics release

Issues: [#1497](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1497),
[#1498](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1498),
[#1499](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1499),
[#1500](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1500).
Issue bodies and all comments were read on 2026-10-14 UTC; all four had zero
comments at investigation. Related old quote/source verifiers are retained.

## Implementation

- Bare Trade selects a genuine eligible factory market and stays in the advanced
  workspace. Explicit identities and bounded prefills win. Repeated active Trade
  navigation preserves a deliberate market and browser history. Native Swap's
  exact configured USTC→CL8Y default remains separate.
- Swap keeps one fixed summary/status region below the inputs and one persistent
  DETAILS disclosure. Disconnected valid quotes display their checked minimum;
  anonymous display does not permit signing. Costs and actual operations stay
  aligned, and the LUNC network fee appears once.
- Fresh same-intent output/route/costs remain during recoverable refreshes while
  signing is blocked. Original expiry and genuine negative/context changes revoke
  stale display/payloads. Missing scheduled router/provider observations are distinct
  from authoritative incompatibility. A stable publication anchor avoids observer
  renders postponing a ten-second refresh toward expiry.
- Order slippage independently compares selected original fee-free execution with
  chained original spot output. Pool/book commissions, taxes/hooks/wrappers and
  independent reference divergence remain separate. Signed favorable values,
  canonical amounts, reserve/book replay and original-cohort binding are validated.
  Original fair-reference 5/30/99 safeguards and exact-net ranking remain intact.

## Evidence and retained limits

All 16 required/relevant source verifier commands completed with passing available checks. Explicit unavailable acceptance remains separate; deployment and sustained live acceptance are the next release steps. [Source receipt](source-verification.json) records scopes and failed-run retention.

Focused checks already passed: Trade entry/navigation; 164 lifecycle frontend
checks and two backend classification/recovery cases; 102 shared metric/frontend
checks, six independent Rust arithmetic cases and type checks. The lifecycle
fake-clock test forces 99 unrelated renders before the ten-second refresh.

Owned funded pool fixture: 24 successful cases across legacy/route-local-v1,
one through four hops and three exact raw sizes. Actual included gas and signed
fee/messages, independently calculated delivered output and rejected-floor atomic
rollback were checked. Complete current DEX contract input equivalence covered
170 files and tracked artifact hashes. See [pool receipt](funded-offline-pools.json). This does not re-attest the original build
recipe, native/tax/hook/book/physical-wallet worst cases or mainnet settlement.

Read-only Grafana/Loki before deployment returned zero error-pattern lines for
both services, with provider-capacity-pattern activity. No Prometheus datasource
is provisioned; this is explicit unavailable monitoring scope, not a pass.
Aggregates alone cannot establish quote/deployment readiness.

Retained first failures: the initial #1498 verifier had obsolete disclosure
selectors and failed browser fixture initialization/refresh/expiry checks while
source was still changing. A later browser retest passed 32/36 checks and exposed
four real postponed-refresh cases; the stable observer anchor fixes that defect.
The initial Rust metric test incorrectly expected 10 rather than 15 taker-fee bps;
its corrected independent formula passed. Initial LocalTerra provisioning had a
root-owned tmpfs, then the image's default insufficient genesis funding; the owned
fixture was reset with UID1000 and the repository's explicit funded genesis,
after which all 24 cases passed. These failures are retained and never relabeled
as successful acceptance. No production/shared infrastructure was reset.

## Acceptance tracking

Individual checklists remain at [Trade entry](../issue-1497/README.md),
[Swap layout](../issue-1498/README.md), [quote lifecycle](../issue-1499/README.md)
and [order metrics](../issue-1500/README.md). Invariants: [TE1497](../../../skills/AGENTS_TRADE_ENTRY.md),
[SL1498](../../../skills/AGENTS_SWAP_LAYOUT.md), [QL1499](../../../skills/AGENTS_QUOTE_LIFECYCLE.md),
[OS1500](../../../skills/AGENTS_ORDER_SLIPPAGE.md).

## Additional completed owned checks

The [production-build geometry receipt](browser-geometry-source.json) passes all
36 checks across 1440/320/390/430 widths, including keyboard, actual route identity,
disconnected minimum, one fee row, delayed refresh, negative/missing/recovery,
original expiry/new intent and 200% text zoom. It records zero provider requests,
28 local quote requests, no page errors, exact built JS hashes and dirty-source
provenance; it does not claim deployed acceptance.

[Native first/retest summary](native-funded-summary.json) records 24 delivered-net
settlements and 48 actual high-hop/high-final-floor atomic rejections. Both trials
use the same 170 DEX source inputs and 47 wrapper inputs from reviewed companion
revision `519c2766e5be0a3788a4c920ad879b641a50aa74`, independently built with locked
Rust 1.83. All compared economics and artifacts remained equal. Native gas was
2.527–2.531M and wrapped gas 2.121–2.122M under synthetic 12M requests; these are
not production gas calibration. [Independent arithmetic](native-metric-arithmetic-first.json)
compares native/matching-credit/equal-gross predictions against each observed
cohort. A separately executed same-cohort matching-credit control, native-output,
tax/hook/book populated fixtures and physical signing remain distinct incomplete
scope. The existing inspected external main/artifacts lacked the current direct
floor protocol; the reviewed companion source was built in an isolated worktree.
Only the owned native container was cleaned up.

The stable-budget pool first/retest also passed 24+24 cases on new owned contracts;
original contract inputs and the offline-budget source hash did not change across
the runs. Subsequent advisory book binding changes alter no transaction messages,
contract bytes or gas-family budgets; those already completed actual receipts are
preserved separately from the final source verifiers' optional funded skips.

A final advisory consistency regression changed an unused maker row while
preserving original execution receipts, producing an incorrect favorable metric.
The bounded original maker terms now bind the counterfactual, including unused
rows; its failed synthetic input and passing regression are retained. Original
fair-risk/signing controls were unaffected. Manual-plan diagnostic policy is also
kept aligned with its final execution policy; a proven mismatch was fixed at the
classification point. These were working-tree findings fixed before release.

## Final available source verification

Passing available checks: `1497`, `1498`, `1499`, `1500`, `1483`, `1489`, `1473`,
`1480`, `1452`, `1479`, `1398`, `1381`, `1390`, `1396`, `1464`, `1453`.
Source/type/lint/format checks and original signing/fee/floor/tax/launch/reference
controls passed. Explicit optional live/device scopes remain unchecked.

The full [controlled browser matrix summary](browser-matrix-source-summary.json)
is complete, untruncated: 4,250 first/retest rows, 3,948 useful quotes, 302 typed
blockers, zero failures, 1,988 local quote requests and zero provider requests.
Useful original-input-through-render p50/p95/p99/max were 80.8/138.5/157/550.6ms
against the specified 750ms browser budget. The full raw receipt is retained
privately with its published hash; these synthetic fixtures use zero-fee wrappers
and do not claim deployed economics or settlement.

The #1473 run's only failed check was a documentation literal removed while
reconciling Trade defaults. Its exact documentation function passed after restoring
native USTC wording; all unchanged executable checks and the full browser matrix
were already passing, so that documentation-only correction did not erase or
relabel the failed first command. The authenticated #1396 fixture also initially
failed because our disposable cluster retained default PUBLIC connection to its
control/template databases; those owned ACLs were repaired and its full verifier
retested with 28 PASS, zero FAIL. Production databases were untouched.

Tax guidance now lives in DETAILS, keeping route-funding wording out of Pay's
height. All 122 affected page/receive/tax tests and the tax-disclosure placement
assertion passed; the final geometry and first/retest #1498 verifier passed on
those final source bytes. Original quote economics, contract bytes and budgets
were unchanged by this final placement correction.

The #1381 driver also executed funded route-tax migration, funding and atomic
invoice/token/native flows successfully. Its raw receipts include synthetic test
actors/contracts and remain private; no production custody is claimed.
