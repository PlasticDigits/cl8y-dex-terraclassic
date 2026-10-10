# #1452 follow-up implementation and verification

Current source coverage after #1458: [issue #1459 implementation and checklist](../issue-1459/README.md).
The #1458 scope and receipts below are historical; #1459 supersedes the direct-wrapper
protocol/signing limit, uncensused wallet admission and composed inverse rejection.
Nonzero cooldown/AutoLP remains explicitly unsupported where required public state is absent.


Implementation: [follow-up PR #1458](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1458).

Related: [issue and comments](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1452),
[agent checklist](README.md), [LQ1452 invariants](../../../skills/AGENTS_LOCAL_SWAP_QUOTES.md),
[tax scope](FOLLOWUP-TAX.md), [exact modes](FOLLOWUP-MODES.md), and
[incremental ingestion/recovery](FOLLOWUP-INGESTION.md).
This supersedes the initial implementation's unsupported-scope list in
[RESULTS.md](RESULTS.md); its historical receipts and failures remain preserved.

## Result and evidence

**PASS WITH SKIPS for the local implementation review.** The focused verifier
itself passed **14 groups, 0 failures, 0 skips**, including its test/retest and
unchanged-source check. Production, device and complete issue acceptance remain
separate. [Machine receipt](followup-local-results.json) records the exact source
fingerprint, UTC times, commands, counts, generations, cache counters and latency.

Verified code revision: `c455598c7c48dd709b49c24645486ec44175f2f4`. The receipt commit only updates documentation.

Verification ran from **2026-10-09T13:30:54Z** through **13:36:47Z** on base
`c131e63f6fcac6d5bcc85a524105c14ad6f46a9d` plus this follow-up's source. Fingerprint:
`9497fd5c3555f1c318275f715b5b1ca424319971007321e7f94597ad48478343`.

Each pass ran 659 library tests, the explicitly invoked ignored discovery
benchmark, 79 API tests across six disposable-DB binaries, 339 frontend tests in
25 suites, and 40 actual browser input/result measurements. The API count includes
one independent database-target safety assertion per binary. The full frontend
unit suite separately passed **3,097 tests**, and the unchanged contract workspace
passed **999 tests**. Types, lint, formatting and crosslinks passed; lint retains
five pre-existing warnings outside this change.

| Initialized socket case | First p50 / p95 / p99 / max (ms) | Retest p50 / p95 / p99 / max (ms) |
| --- | --- | --- |
| 67 exact response-cache misses | 3.063 / 5.115 / 8.211 / 20.998 | 2.595 / 3.078 / 3.161 / 13.995 |
| 92 warm/concurrent requests | 1.228 / 60.646 / 60.709 / 61.224 | 0.883 / 60.390 / 60.457 / 60.935 |
| Useful baseline / later improvement | 2.864 / 7.270 | 2.906 / 6.556 |

All final measured initialized results were strictly below 250 ms. Browser
retest maxima were **61.2 / 37.5 ms** for desktop Swap/Trade and **44.2 / 37.0 ms**
for phone Swap/Trade. Quote-stage provider calls, browser chain attempts and page
errors were zero. [Current full browser transport receipt](browser-controlled.json)
and [both verifier summaries](followup-local-results.json) retain every sample.
These are normal loopback HTTP measurements against synthetic indexed observations,
with a production frontend build, disconnected wallets and separately measured
cold initialization. They do not establish deployed performance.

Cold fixture admission uses a bounded **512-call burst** for its known synthetic
population; both runs observed **419 calls** before provider isolation. This is
an explicit unpaced fixture allowance, not production ingestion or cold-start SLA.
Production retains two outstanding operations, four request starts/second and
its original deadlines. Mapping and manager-context readiness are checked before
providers throw, without warming the measured executable route keys.

## Completed work

- [x] Locally funded v1 tax forward/pool receive sizing, exact sell residue and
  post-hook buy net; immutable config/identity and observed actor/cap checks.
- [x] Exact manual POST budgets on every hop, cache separation, no deferred plan
  substitution, and signed budget/maker/hint/floor binding.
- [x] Hook inverse residue indexing with bounded corrections for AMM output jumps;
  full-forward sufficiency protects composed routes and healthy alternatives.
- [x] Zero-hop wrapper arithmetic and receive sizing, explicit signing refusal
  where external contracts cannot enforce the reviewed output floor.
- [x] Incremental scheduled admission, original per-pair expiry, bounded actor
  rotation and complete pinned-height inventory recovery with epoch protection.
- [x] Actual CW20/native payload serialization selects funding version 1, forbids
  unobserved replacement recipients and makes no quote/preparation smart queries.
- [x] Migrated forward, reverse, tax, hybrid, health and page fixtures; retained
  independent arithmetic, URLs, decimal scaling, wallet guards and signing checks.
- [x] Reusable verifier, explicit skip policy, current budgets and agent crosslinks.

## Issue criteria status

A checkmark means the complete listed criterion; narrower evidence is listed
without asserting complete acceptance.

- [ ] AC1: supported thin/deep/book, mixed-decimal and native fixtures pass;
  complete all-market/native/tax/actor corpus remains unverified.
- [x] AC2: initialized admitted direct/two-hop quotes execute locally with useful
  output and zero provider calls. Unsupported cached semantics fail explicitly.
- [ ] AC3: all final controlled miss/hit and desktop/phone measurements pass;
  the production reference environment and external network remain SKIP.
- [x] AC4: useful baseline precedes bounded eligible improvement; no global-optimum
  claim or receive reset. Manual plans cannot be improved into a different plan.
- [ ] AC5: request-stage stubs/signing and independent worker bounds pass;
  complete connected-wallet ancillary-network/deployed ingestion scope remains unverified.
- [ ] AC6: exact amount/target/mode/actor/tier/plan/generation identity and bounded
  cache coalescing pass; largest populated-book graph and concurrency abuse remain narrower.
- [ ] AC7: stable observations, stale/future checks and reserve/config/book/reference
  invalidation pass locally; the complete populated-market refresh corpus remains unverified.
- [ ] AC8: reviewed source display/preparation/payload parity, positive floors,
  Expert/99% guards and races pass; complete device/connected-wallet corpus is SKIP.
- [ ] AC9: explicit missing/unsupported/blocked/dust errors and absence of unsupported
  size advice pass; full deployed blocker corpus remains unverified.
- [x] AC10: focused verification/retest, current work/optimality budgets and source,
  frontend, solver and agent documentation crosslinks are complete.

## Related verification and remaining scope

`make verify-issue-1398`: **7/0/0**; `1399`: **8/0/0**;
`1381`: **13/0/2**; `1390`: **14/0/2**; `1396`: **28/0/2**;
`1422`: **10/0/1**; `1453`: **12/0/2** (PASS/FAIL/SKIP).
Available checks were completed before assigning skips. Actual LocalTerra funding,
rollback and atomic invoice fixtures passed; #1422 repeated real 3M smart queries
and overhead-scoped probe transactions using the pinned SDK53 node. The verified
contract artifacts match all 170 current contract source inputs. No production
transaction, shared database migration or production deployment was performed.

Rollout requires a compatible indexer before the frontend: clients require each
hop's `declared_hybrid` signing anchor (explicit `null` for pool-only). Older
metadata fails closed. Coordinate that deployment under #1456; it is not performed
by this source PR.

Unimplemented semantics remain engineering work: external direct-wrapper floor
support, stateful cooldown/AutoLP economics, tax wallets absent the independently
observed actor census, and composed inverse candidates that fail forward parity.
They fail closed and are not verification skips. Largest-factory freshness and
full loaded-graph latency still need independent evidence. Existing deployment,
provider/history and device work remains tracked in
[#1456](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1456).

Hosted CI status is unavailable: `bao-exec --only FORGEJO_PLASTICDIGTS_PAT -- fj actions tasks`
returned "not found". No successful remote CI status is claimed; this unavailable
publication gate does not hold the user-authorized source PR.

Explicit unavailable checks: production rollout/network/provider/history,
physical signing, the #1381 external wrap stack, the #1422 independently reviewed
frozen before-fix artifacts, and host published-bridge TCP/disabled IPv6. No
human engineering checklist or per-issue operator handoff is required by this PR.

## Failure history and retesting

Executed failures were preserved and corrected/retested. They included stale
fixture expectations, mismatched per-client admission namespaces, progress
completion races, missing exact-plan metadata, the funding serializer defect,
and the formatter's hard-coded Rust edition. Discovery regression assertions now
use proven typed rejection evidence and bounded refill handling, without widening
runtime budgets. Synthetic database PUBLIC grants were corrected only in the
new disposable cluster. Cold setup now proves required mappings/actor observations
and asserts its bounded call population before request measurements.

[Loaded browser failures](followup-browser-loaded-failures.json) retain all samples,
including the 1,208 ms desktop outlier and 445.1/401.6 ms phone outliers. They remain
FAIL; the final successful runs do not establish an all-load or production SLA.
Earlier concurrent liquidity/provider/browser tests failed before passing quiet
full verification. LocalTerra's first run hit a transport EOF after two passing
scenarios; its complete retest passed. The gas probe initially failed linker/VM
compatibility before a compatible Rust 1.83 fixture build passed both actual-query
runs. A preliminary verifier was stopped after its source fingerprint changed;
the final unchanged-source run above completed fully.

## Instructions for the reviewing agent

1. Use an isolated worktree and Node wrapper; provide private disposable QA
   credentials under #1396. Read the three linked follow-up invariant documents.
2. Run `make verify-issue-1452`; its repeated tests must pass on an unchanged
   fingerprint. Inspect arithmetic, zero-RPC, cache, expiry, plan and payload
   assertions. Run the available related verifiers listed above.
3. Inspect exact signed funding-v1 messages and immutable manual budgets. Keep
   legacy zero-hop signing blocked; review protected protocol signing under #1459.
   Unknown actor/cooldown/AutoLP semantics cannot fall
   back to gross, wallet funding, live validation or weaker floors.
4. Record revision/UTC, command, counts, generations, counters and all latency
   samples/maxima. Correct executed failures and retest before checking an item.
5. If a prerequisite is unavailable, mark that item **SKIP**, state the attempted
   command and precise missing capability, and leave its acceptance box unchecked.
   Overall **PASS WITH SKIPS** is allowed when every executed check passes.
   Failed assertions remain **FAIL**; workflow overrides never waive runtime guards.
