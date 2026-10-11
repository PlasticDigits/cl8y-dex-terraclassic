# Issue #1473 implementation and verification

Issue: [#1473](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1473).
The fix PR and final source revision will be recorded after publication.
Follow [the agent procedure/checklist](README.md) and
[TQ1473](../../../skills/AGENTS_TRADING_QUOTE_OUTCOMES.md).

## Behavior and invariant review

The amount-dependent rejection was real: the UI's quoted-output tolerance was
also used as a pool-spot spread bound, causing a larger trade to reject its own
accurately computed price. Retail now explicitly requests `quote_relative_v1`.
Its exact selected percentage protects post-hook/tax net receive with positive
rounded-up hop/final floors. Checked pinned market economics derive a separate
finite L9 ceiling, which the client independently verifies before signing.
Existing router messages support this behavior; no contract migration is needed.

This applies to arbitrary supported execution identities and exact amounts, not
a seven-pair admission whitelist. The default alone is native `uusd` → configured
CL8Y. Explicit links, requested funding direction, user choices and genuine
factory/chart/limit identities retain precedence. Retail shows one executable
receive quote, with no competing indicative estimate or extra pricing panel.

The source also binds the official response policy, rejects malformed or
contradictory Boolean capability claims, and checks the original 20-second
window at validation and mutation. Typed public errors stay within 4 KiB; raw
remote/private prose is omitted. Known pricing errors recover on independently
changed local generations, without endless unchanged solve retries or any
request-triggered RPC. Legacy API requests retain their previous L9 policy.

## Temporary user-directed browser budget

On 2026-10-11 UTC the user explicitly requested **750 ms for now** for browser
latency verification. New controlled and deployed browser trials use the named
`browser_latency_budget_ms: 750` criterion from original input through confirmed
useful render, including all attempts. The fresh controlled full first/retest
trial passes that criterion as recorded below. Socket latency stays at 500 ms;
UI protection percentages, integer floors, original expiry and resource limits
are unchanged.

Every previously executed 500 ms failure below remains FAIL. The historical
679.7/556.7 ms trial is not reclassified as PASS under the new threshold; it must
be followed by fresh runs. AC7 in the issue body now explicitly states the
temporary 750 ms criterion; prior 500 ms acceptance remains unestablished rather
than becoming a passing historical result. Incomplete historical/deployed/device/
security acceptance keeps the issue open. Any final issue comment must state the
current criterion and override explicitly; closure still requires every current
body/comment criterion to pass.

## Executed evidence

The available first/retest source scope includes 91 local-quote tests with the
420-case shared integer corpus, 30 topology, 10 hybrid, 10 tax and 15 exact-output
API tests, and all 16 real HTTP socket/fee-generation scenarios twice. The
focused frontend scope passed 424 tests across 28 files twice, plus TypeScript,
ESLint and whitespace checks. Contract clippy and the #1398 focused/full workspace
verifier passed. The final controlled browser first/retest receipt passes the
temporary criterion described below; these individual checks do not
establish complete issue acceptance.

[The final selected-source composite](source-final-750.json) records the core
first/retest at 08:21:47–08:27:40 UTC: 14 PASS, 0 FAIL and 3 SKIP. Its browser
entry was explicitly covered by the separate complete trial on the identical
`7d0df872…` source fingerprint; combining that executed PASS gives **15 PASS,
0 FAIL and 2 SKIP**. The remaining omissions are deployed provenance/sustained
matrix and physical-device/complete-settlement scope, with the owned funded
subset documented below. This selected-scope verdict does not erase the separate
#1396 executed host-bridge failure or establish complete issue acceptance.

[Funded first proof](quote-protection-first.json) and
[funded repeat proof](quote-protection-retest.json) each pass 24 direct/multihop
6/18/9-decimal cases in both directions at 1%, 5% and 30%. Twenty cases reproduce
the old false L9 rejection and then execute the corrected messages with exact
quoted net delivery. Rejected-floor transactions leave reserves and LP shares
unchanged. Contract artifacts were checksum-verified and their current contract
source inputs matched the reviewed artifact provenance. This is an owned
LocalTerra proof with no production signing and no claim of complete tax/hook or
native-wrapper settlement parity.

An independent read-only review found no confirmed high-impact correctness or
security issue in the integer floors, L9 ceiling, policy isolation, actor/window
bindings, tax funding, native conversion or signing snapshots. Direct unwrap
mutation guards rely on the wallet floor validated against the displayed net
quote: the UI synchronously checks and serializes that same snapshot before its
first broadcast await. This review did not run settlement or browser tests and
does not establish complete acceptance.

The reviewed precommit formatting changed six QA JSON files and the live browser
helper's bytes. Canonical parsed JSON and JavaScript AST hashes before/after were
identical for all seven files; the AST comparison ignores location and literal
spelling only. Matrix byte SHA-256 changed from
`ad1482002eddcc73a852aad7e39ee27c78ea7764025c0ac4d6f69e68e6d53608` to
`009cde25524dd4b43db90bded8f9724bb488110c2d260d27a6a49f7e38d23f0e`;
its parsed-data hash remains
`8f609ed62f8623219a66ba88887c54a79a53b8840e30010afd22f2fccd06a43e`.
Historical receipts retain the exact matrix hash they measured. New runs must
record their actual source/matrix fingerprints; byte equality is not claimed.
The subsequently retained corrected-fixture failure JSON was also formatted
with an identical canonical parsed-data hash.

## Failures retained and corrections

Executed failures remain in their original receipts and were never converted
into PASS or unavailable skips:

- Legacy API tests expected discarded generic prose. Updated assertions check
  the exact typed public reason and preserved execution/provider guards. Known
  unsupported capabilities remain deterministic 422; actual missing data stays
  503. Topology-only disconnected routes now say `no_route` rather than protection.
- An original quote lifetime exceeded 20 seconds by two milliseconds under a
  permissive book age. Quote eligibility and expiry now enforce the original
  20-second bound. The deliberate 30/90-second fixed-view tests now require
  expiry rather than silently extending retention.
- Near-expiry dual-view fixtures initially exhausted their lifetime during cold
  setup. The accepted fixture retains original timestamps and expiry inside the
  unchanged four-second fee window, with measured setup headroom. A proposed
  warm-up with missing reserves was rejected and removed because it correctly
  created a negative watermark. Quiet full socket trials then passed twice.
- The first funded-driver wallet delta included protocol commission because the
  recipient was also treasury. The corrected fixture uses distinct synthetic
  recipients and treasury, retaining exact net equality and rollback assertions.
- Early overlapping browser trials produced long delays and malformed native
  fixture plans. The native fixture now uses genuine wrapped factory identities
  and bound wrapper metadata. Isolated desktop/phone native trials pass; earlier
  failures remain in the record.

[The full browser first failure](browser-first-failure.json) retains a stable
4250-scenario run with 3948 useful quotes, zero provider requests, and one measured
797.5 ms failure. Its response arrived 33.8 ms after the original event, followed
by a 752 ms main-thread long task. Useful p99 was 56.5 ms; the maximum still fails
the 500 ms criterion. A focused CPU/timeline trial of the same exact input passed
first/retest in both viewports; a neighboring slower task remains under analysis.
No environmental cause or passing full latency verdict is inferred from that
partial evidence.

[The full browser retest failure](browser-retest-failure.json), recorded at
07:29:54 UTC, again completed all 4250 scenarios with 3948 useful quotes and zero
provider requests. It recorded three desktop useful-output failures: 521 ms for
CL8Y → UST1 at raw `14999965500000000000` and 1%, 1630 ms for cached
USTC → LUNC at raw `4` and 5%, and 1097.4 ms for UST1 → CL8Y at raw `2`
and 30%. Useful p50/p95/p99 were 34.6/51/57.4 ms; the maximum remains a
failure despite the passing distribution. The cached case includes a 1580 ms
main-thread task; the other two show delayed response completion with no observed
long task. These measurements do not establish their underlying cause.

The full source verifier ended at 07:30:10 UTC with 14 PASS, 1 FAIL and 2 SKIP.
Its historical 500 ms browser check remains FAIL. A fresh full first/retest matrix
now passes the explicitly requested temporary 750 ms criterion on stable final source.
These failed runs will remain recorded after any successful correction.

[The corrected-fixture first full failure](browser-fixture-first-failure.json)
completed all 4250 scenarios at 07:59:00 UTC with 3948 useful quotes and zero
provider requests. It used disabled GPU acceleration, native shared memory,
scoped DOM observation and a confirmed render frame. Useful p50/p95/p99/max were
101.8/174.9/214.6/679.7 ms. Desktop CL8Y → USTC at raw `28500000541501`
and 30% took 679.7 ms, including a recorded 480 ms main-thread task; phone
cached USTC → LUNC at raw `3` and 30% took 556.7 ms without a recorded long
task. Both failed the original 500 ms ceiling in that executed trial. The full
failure retains its original pre-format matrix/source fingerprints and a digest
of the private full
receipt. These changes alone did not resolve the original 500 ms target. No
graphics, polling or shared-memory cause is inferred from this failed trial.

## Fresh controlled browser result under the temporary criterion

[The bounded summary](browser-750-pass-summary.json) and
[full per-case first/retest receipt](browser-controlled.json), recorded at
08:20:20 UTC, contain all 4,250 scenarios: 3,948 useful executable quotes,
302 bounded rejections, zero failures under the explicit 750 ms budget,
1,989 quote GETs and zero provider requests. Chrome 151.0.7922.137 measured
the original input event through confirmed visible render-frame completion.
Useful p50/p95/p99/max were 84.7/148.8/183.5/528.9 ms.

| Viewport and cache class | Useful count | p50 ms | p95 ms | p99 ms | Maximum ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Desktop initialized HTTP | 796 | 109.7 | 169.7 | 219.7 | 524.1 |
| Desktop browser cache hit | 1,178 | 98.4 | 151.2 | 194.5 | 528.9 |
| Phone initialized HTTP | 795 | 83.1 | 105.7 | 116.4 | 382.3 |
| Phone browser cache hit | 1,179 | 68.9 | 86.3 | 95.2 | 468.1 |

The source fingerprint before and after the fresh full run was identical:
`7d0df872c5d2f1b041f6e6f241973477aa6de6bd5d4235cdfce1ad9c3cde980b`.
The run uses the final formatted matrix hash recorded above. Three useful
measurements exceeded the original 500 ms threshold; this passing 750 ms trial
does not establish the original 500 ms target. All earlier failures retain their
original verdicts. After matrix formatting, the shared Rust matrix and focused
integer-floor arithmetic tests both pass, and all six probe safety tests pass.
This controlled receipt does not establish deployed, device or complete
settlement acceptance.

## Acceptance status and remaining scope

- [x] AC5 source API/UX contract: fixed allowlisted reasons, bounded privacy-safe
  mapping, explicit official policy, distinct HTTP-200 validation failures and
  current local-engine documentation.
- [ ] AC1 historical production economics: baseline attachments retain exact
  offers/results but not the original reserve/reference observations. Synthetic
  parity and current live observations do not reconstruct those historical pins.
- [ ] AC2 full supported-state/capability acceptance: generic source pricing and
  shared corpus are verified; complete deployed actor/hook/wrapper coverage is separate.
- [ ] AC3 complete user/state acceptance: focused intent/generation tests pass;
  remaining deployed and receive/manual/browser race scope needs its receipts.
- [ ] AC4 complete entry acceptance: source permutations, native navigation and
  funding-orientation tests pass; final deployed frontend provenance/default trial remains.
- [ ] AC6 complete expanded source/socket/browser/deployed first/repeat matrix.
- [ ] AC7 complete initialized browser latency: the fresh controlled full
  first/retest passes the temporary 750 ms criterion; complete deployed
  acceptance is separate. Prior 500 ms failures retain FAIL.
- [ ] AC8 complete settlement/security/provider/QA acceptance: listed source and
  funded proofs pass; tax/hook/native/device and host networking limits remain explicit.
- [ ] AC9 separate exact frontend/backend deployment, classified logs and matrix
  repeats beyond observation/configuration cycles.

The host #1396 custom-bridge published-loopback test executed and failed, even
with an owned non-overlapping IPAM network and a correct bridge route/proxy.
The existing default-bridge QA path works; owned internal SQL, namespace denial,
and the supported explicit-host IPv4 path pass. Host loopback IPv6 is unavailable.
No shared daemon, routes, ACLs or service volumes were changed to mask this failure.
This failed scope remains distinct from passing disposable database isolation
and cannot establish complete acceptance.

Unavailable physical devices, historical pins or deployment artifacts may be
explicit SKIP with **PASS WITH SKIPS for the stated tested scope**, following
the checklist. Executed correctness/latency failures remain FAIL until corrected
and rerun. The issue stays open unless every body/comment criterion passes.
