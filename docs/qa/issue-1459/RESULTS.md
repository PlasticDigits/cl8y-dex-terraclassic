# Issue #1459 source results

**PASS WITH SKIPS for local implementation verification.** All executed final
checks pass. The local-quote verifier passed **14 groups, 0 failures, 0 skips**
on unchanged sources, including its complete test/retest. The six applicable
verifiers total **87 passing groups, 0 failed groups, 5 explicit skips**.
[Machine receipt](verification.json), [review checklist](README.md),
[QC1459 invariants](../../../skills/AGENTS_LOCAL_QUOTE_COVERAGE.md), and
[companion protocol PR #62](https://git.cl8y.com/code/ustr-cmm/pulls/62).

Source worktree: `fix/issue-1459-quote-semantics`, based on
`736be444100ba84887a6619336c83d2848258884` plus the reviewed change.
Final source fingerprint: `76ccd473158bed56e37d47b005cf136c5f690df24da1a2703fc81185b40f07f1`.
Final quote verification began at **2026-10-10T01:35:01Z**;
commands, UTC times, generations, counters and all distributions are in the receipt.
The companion protocol revision is `519c2766e5be0a3788a4c920ad879b641a50aa74`.

## Issue criteria completed

- [x] **C1 — direct wrapper floors:** compatible treasury/mapper expose version 1
  capability and distinct protected messages. Scheduled admission enables exact
  signing only for that protocol; legacy/unknown/expired observations remain
  nonexecutable. Actual recipient balance-delta replies enforce positive floors
  and atomic rollback. Wallet floor, input, direction, actor and generation stay
  pinned; signing uses offline gas with no preparation smart queries.
- [x] **C2 — stateful tax alternative:** nonzero cooldown and enabled AutoLP remain
  explicitly unsupported because public observations lack last-trade and complete
  trigger/settlement state. Manager exemption cannot waive this. A newly failed
  required asset observation immediately removes selected and unselected related
  pairs; stale supported slices cannot reintroduce that asset.
- [x] **C3 — uncensused wallets:** bounded scheduled holder pages and configured
  zero-balance buyer/manager enrollment work without a trader-census row. Independent
  wallet ledgers survive pair replacement/pages; original factory/token timestamps
  bound expiry. Caps, protocol/blacklist negatives, partial-read failures and
  capacity quarantine remain enforced with zero request-triggered discovery.
- [x] **C4 — composed inverse:** the formerly insufficient candidate now refines
  to a sufficient smaller input than its alternate. Independent integer tests
  prove first-reaching inputs through upstream/downstream hook dips, commissions
  and three hops. Whole-route forward validation and finite work remain mandatory;
  exhaustion preserves eligible alternatives and never publishes an insufficient
  seed. This is bounded evaluated-path economic minimality.

C2 completes the issue's explicit unsupported-admission alternative. No dynamic
cooldown/AutoLP execution model is claimed. C1 source support requires deploying
both companion contracts before the old external protocol can advertise it.

## Final verification

| Command | PASS / FAIL / SKIP |
| --- | --- |
| `make verify-issue-1452` | 14 / 0 / 0 |
| `make verify-issue-1398` | 7 / 0 / 0 |
| `make verify-issue-1381` | 13 / 0 / 1 |
| `make verify-issue-1390` | 15 / 0 / 1 |
| `make verify-issue-1422` | 10 / 0 / 1 |
| `make verify-issue-1396` | 28 / 0 / 2 |

Each local quote pass ran **686 library tests**, the separately invoked ignored
benchmark (**1 test**), **81 API tests across seven disposable-database binaries**,
**412 frontend tests in 27 suites**, and **40 browser input/result measurements**.
The API count includes seven independent database-target safety assertions.
The final complete frontend suite passed **3,105 tests in 358 files**; the unchanged
contract workspace passed **999 tests**. Types, lint, formatting and whitespace
checks pass. Existing backend dead-code warnings and five frontend lint warnings
remain outside this implementation.

| Initialized socket case | First p50 / p95 / p99 / max (ms) | Retest p50 / p95 / p99 / max (ms) |
| --- | --- | --- |
| 67 response-cache misses | 2.616 / 3.354 / 4.645 / 34.341 | 2.479 / 3.019 / 3.036 / 12.424 |
| 92 warm/concurrent requests | 0.881 / 57.036 / 57.108 / 57.596 | 0.844 / 48.090 / 48.143 / 48.566 |
| Useful baseline / deeper improvement | 3.275 / 6.578 | 2.788 / 7.095 |

Browser maxima were **64.7 / 35.7 ms** desktop Swap/Trade and
**48.1 / 38.5 ms** phone Swap/Trade; retest maxima were
**47.5 / 37 / 43.7 / 36.4 ms** respectively.
[Both browser passes](browser-passes.json) retain all samples; the
[complete final browser trace](browser-controlled.json) retains DOM/request traces.
Quote-stage provider calls, browser chain attempts and page errors were zero.
Cold synthetic admission used **423 HTTP reads**, within its explicit 512-read
fixture burst; production provider/cadence/concurrency budgets were unchanged.
Each socket pass recorded **92 hits, 67 jobs started, 0 active jobs and 67 results**.
These are initialized controlled loopback measurements with synthetic observations
and a production frontend build, disconnected wallets, and separately measured
cold initialization. They do not establish a deployed or all-load SLA.

## Chain and protocol evidence

The companion treasury passed **146 tests** and mapper **113 tests**, with repeated
focused/full verification. Two isolated SDK53 LocalTerra wrapper fixtures proved
positive-floor ABI, actual bank-net delivery and deposit/mint/send/burn/withdraw/
supply/rate rollback. Wrong floors failed for the exact expected reason; exact
floors passed afterward. [DEX-token wrapper receipt](localterra-wrapper-receipt.json)
records hashes, public identities and transaction receipts. Wrap used **459,431 gas**;
unwrap used **500,258**, exact-net retry **500,607**. Nominal native output **497,500**
delivered **495,013** after the actual bank deduction. Protected source gas budgets
are **800,000 / 1,200,000**. The old 400,000 wrap budget was insufficient.

Actual [funded tax scenarios](localterra-tax-funding.json),
[native/invoice flows](localterra-native-invoice-flows.json) and
[launch guards](localterra-launch-guards.json) passed. The existing native wrap
stack was provisioned and exercised; it was not skipped. Both
[3M first](localterra-3m-first.json) and [retest](localterra-3m-retest.json) runs
passed all actual smart queries at the unchanged 3,000,000 limit. Probe transaction
gas includes wrapper/transaction overhead and is separately scoped.
No production transaction, application-database migration or shared-volume reset
was performed. These synthetic artifacts/receipts do not replace release provenance.

## Explicit unavailable scope

- [ ] Production treasury/mapper/indexer/frontend rollout and release provenance;
  deployed capability, execution and loaded/reference-network/connected-wallet/device
  acceptance remain on [#1456](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1456).
  The two production-verifier skips do not create source publication holds.
- [ ] Independently reviewed frozen before-fix #1422 artifacts were unavailable;
  actual current-artifact 3M checks were completed twice.
- [ ] Default Docker bridge published-loopback transport: a fresh internally-ready
  Postgres 18 preflight reset its host startup connection. Real container-side,
  host-network and confinement checks passed; that host transport remains SKIP.
- [ ] Host IPv6 query: `/proc/sys/net/ipv6/conf/lo/disable_ipv6` is `1`; remaining
  socket probes completed. No global host network setting was changed.
- [ ] Authoritative token-context recovery after tax-ledger exhaustion remains
  unsupported. Factory inventory cannot clear a different token-role quarantine.
  Partial enumeration or verification skips never clear runtime guards.

Historical reconstruction/active provider and DeFiLlama scope remain on #1456
and [#1449](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1449).

## Failure history and agent retest instructions

Executed failures were corrected and retested. They included inverse/protection/
standalone-reference fixture assumptions, a transient shared-file compile race,
and missing scheduled `all_accounts` fixture responses that opened the provider
circuit. Empty supported pages plus zero-deferral/closed-circuit assertions fixed
all API failures using original provider budgets. A full frontend retest exposed
AC7 waiting for default selectors before actual URL application; it now waits for
canonical URL and asserts its reversal too. The final complete frontend suite and
unchanged-source quote verifier passed afterward. Preliminary stopped/failed runs
were not relabeled as skips; earlier loaded failures in #1452 remain preserved.

Reviewers should follow [the criterion-by-criterion agent procedure](README.md),
run the six applicable verifiers, inspect payload/rollback/budget/expiry assertions,
and record revision, UTC, counts and raw samples. Correct any executed failure and
repeat affected checks. An unavailable item may be explicit **SKIP** with attempted
command, concrete missing capability and unchecked acceptance; overall
**PASS WITH SKIPS** is permitted when all executed checks pass. Runtime controls,
unimplemented semantics and deployed acceptance cannot be waived by that policy.
