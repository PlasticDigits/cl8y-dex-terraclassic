# Route discovery results (#1489)

Implementation: [PR #1490](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1490).

Issue: [#1489](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1489).
Follow [the checklist](README.md) and
[RD1489](../../../skills/AGENTS_ROUTE_DISCOVERY.md).
Related raw-economics evidence: [#1483](../issue-1483/RESULTS.md).

## Investigation

On original reviewed source `4cb5736574072130e8e14a5ea2e297d3062e8212`,
automatic native 10,000 LUNC selected two pool hops while a declared three-pool-hop
quote was eligible on the same original amount/cohort. The issue retains exact
raw input/output, generation, observation, expiry and floors. This evidence is
bounded to the independently recorded case and does not certify settlement.

Source inspection confirms two failure stages: successful producer validation
unconditionally advanced a global revision, invalidating otherwise compatible
deeper work; cached short results returned before retrying deferred deeper work
when four active jobs had previously exhausted capacity. Routine configuration
publication also had an epoch gate in addition to the full economics/policy
generation check. Changed authoritative fees and required-read negatives remain
real invalidations and must be independently covered by regressions.

Required constraints remain: four pool hops, finite path/edge/job/cache bounds,
native pool-only semantics, exact policy/amount/context, original expiry,
independent references and all runtime funding/floor/guard controls. Wrapping
does not consume a pool hop. Truncated search must not claim a global optimum.
No address-specific routing, reference-floor change or cap increase is justified.

## Verification and deployment

Exact final source fingerprint, tests, failed inputs, retests, deployed revisions,
raw outputs, timings and bounded monitoring receipts are recorded below as work
completes. Unavailable evidence stays **SKIP** with unchecked criteria; executed
failures remain **FAIL**. Passing implementation subsets cannot certify complete
issue acceptance, independent production SQL or physical wallet settlement.

## Original routing implementation verification

[Source receipt](source-final-summary.json) records **20 PASS, zero FAIL and two
external-acceptance SKIPs**, repeated on exact unchanged product/tool fingerprint
`92dff16a357c80f9c1d713061b6bfc0d13d2ea89bbcb9828bf9174173eb92c17`.
Both rounds pass nine new backend regressions, the owned original-cohort socket
fixture, 302 focused frontend tests, seven probe controls, two monitoring controls,
nine private wallet/scope controls and six actual browser-tool boundary controls.
The full frontend separately passes 370 files/3,382 tests per round, with unchanged
frontend fingerprint; lint has zero errors and five existing warnings. The build
with actual public production configuration passes, without credentials in the
build child. Related verifier and live/deployed results are separate below.

The scheduler keeps four active workers, coalesces FIFO deferred intents within
256 entries and a conservative 16MiB snapshot/book/context accounting budget,
shares reviewed snapshot references and avoids allocation on coalesced cache hits.
Release drains deferred work without another client request. Expired/removed or
invalidated work cannot publish. Unchanged positive fee completion no longer
advances an economics revision; authoritative changed fees/negatives, qualification
and phase invalidation remain. Routine identical configuration epochs do not
revoke equivalent work, while tightened validity windows advance a monotone
validity revision and cannot restore an old cache key on later refresh.

Promotion ranks exact eligible delivered net and caps its original observation,
expiry, snapshot age and attribution at the earlier baseline/candidate window.
Native requests remain pool-only even when the flag is omitted; declared native
hybrid plans fail unsupported while wrapped book controls retain their superior
book route. Selected-route wrapper limits/reference safety are applied before
winner selection. Receive sizing evaluates at most 64 inverse paths per pass,
retaining the existing bounded longer-path fallback on initial failure. It reports
its minimum-gross-input objective, rather than claiming net-output ranking.
Path-cap truncation is conservative at exactly 1,024 paths; no extra exhaustion
probe exceeds that cap. Work remains 16,384 enumeration visits/four pool hops.

Progress observes the exact original native/gross/protection/mode/context intent.
Same-generation higher-net signals request normal validated quotes in Swap/Trade;
review/signing and declared/receive paths stay pinned or outside the observer.
One selected-pool price-movement notice preserves exact 5/30/99 controls. The
three-second promotion bound is an initialized verification target, not an
unconditional CPU-scheduling guarantee; original expiry always wins.

## Retained independent production observations

[Initial deployed counterexample](original-live-api.json) independently compares
native 10,000 LUNC automatic `63386134401366040994` raw USTR with eligible declared
three-hop `83864895115238012908` on the full original cohort. Later automatic POST
and best improve to `84048266732619771357`; this demonstrates intermittent
promotion, not permanent missing-hop support. These differ from the issue's
original 02:41 UTC amounts and are not substituted for that historical input.
Initialized useful sockets have four samples, p50/p95/p99/max
276.526/277.541/277.541/277.541ms at the 500ms budget.

[Original sustained run](original-live-three-cycles.json) retains an executed
same-cohort inferior-best failure beyond the 180-second cycle. Its development
tool changed during collection, so it is explicitly not final-tool attestation.
[Independent semantic follow-up](original-live-semantic-final.json) compares
recognized all-null pool policies canonically while preserving full original
amount/generation/time/expiry/fair/funding evidence. An initially inferior
automatic POST eventually improves; the recorded 0.837s is only the post-prefix
comparison interval, not the total original-input clock. Historical receipts do
not certify behavior after the fix. Policy, original expiry and quote validation
are never weakened to force a same-cohort comparison.

## Owned three-hop settlement

[Final funded first](funded-three-hop-first-summary.json) passes **18 exact-net
settlements and 72 intentional atomic floor rejections** on fresh owned state:
native 1/1,000/10,000 LUNC gross and equal-gross wrapped controls at 1/5/30% floors.
Each case independently checks all three pool reserve/fee-recipient changes,
wallet net, wrapper custody/supply/rate and zero router/intermediate-wallet residue.
Each of three hop floors and the final floor is raised above expected net and
rejected with all economic state restored; paid ante fees are excluded.
Requested 12M synthetic gas and observed toy-fixture gas are separate from
production calibration in #1480. Wrapper source/artifact hashes identify the
owned locked Rust1.83 unoptimized build; they do not attest deployed contracts.
Physical signing, independent atomic production SQL and production settlement
remain explicitly unverified. The repeat receipt follows separately.

[Independent funded retest](funded-three-hop-retest-summary.json) passes the same
18 settlements/72 rejections on a separately initialized owned chain. Expected
and delivered raw economics, every original/final reserve, floor and rejection
match. Wallet sets differ; deterministic contract addresses repeat across the
fresh chains, so this is not a funded contract-address-permutation claim.
Both rounds use driver hash
`482543f479e30465ca660fcacc314a2d0c59ff3475186d94bfd6096da5ea3ab4`.
The initial fixture wrongly mounted `/root/.terra` while the image's actual home
is `/home/terra/.terra`; known pending inclusion was reconciled, then only owned
fixtures were reset onto verified tmpfs before these final passes. No shared or
production state was reset, and accepted/unknown transactions were not resent.

The first related #1452 run retained a real compatibility assertion failure in
`api_route_exact_output`: its minimum-input description omitted local scope.
The description now explicitly says eligible local pool paths; the objective,
original strict assertion and execution guards remain. Fresh owned child DBs and
controlled browser fixtures repeat the corrected final source. Earlier source
fingerprint failures remain recorded independently of final repeated passes.

Implementation worktree/branch:
[fix/1489-route-discovery](https://git.cl8y.com/code/cl8y-dex-terraclassic/src/branch/fix%2F1489-route-discovery).
The final PR, merged/deployed artifact receipt and issue acceptance comment
provide publication/release crosslinks after all feasible source checks pass.


## Resource admission and owned infrastructure retest

The waiting queue's conservative16MiB budget applies only to waiting work.
Already free original workers accept a qualified larger context after older FIFO
work is drained, within the unchanged four-worker cap. A ninth regression proves
this with a valid larger actor/context census: it cannot permanently strand a
short quote merely because its waiting estimate exceeds16MiB.

[Retained controlled timing failures](controlled-browser-initial-failure-summary.json)
record nine failed750ms clocks in the first4,250-case controlled matrix. All
3,948 useful cases passed shape checks; timing failures are not reclassified as
unavailable. Owned browser storage/reset and final-source first/retest repeat
original clocks; their final verdict is separate below. A later API retest also
failed before assertions when the earlier temporary QA control exhausted its
storage and exited. Its shared state was preserved. A new dedicated marked QA
cluster, fresh nondefault credentials and4GiB test-only storage replaced the
capability; each process still owns a child DB, and only verified inactive owned
children are cleaned between suites. No shared migration history or production
state is reset.

## Additional feasible funded integrity controls

[Tax funding first/retest](funded-tax-1381-public-summary.json) adds actual current-
artifact taxed routes across6/18decimals, initial/final and multiple taxed assets,
existing wallet/router balances and atomic final-floor rejection. Exact first/
retest economics agree; legacy artifact migration and production signing remain
unverified. [Oracle manipulation first/retest](oracle-1391-public-summary.json)
executes38included transactions and six reserve/LP-depth stages on owned chains,
then replays both real receipts through the current production HubUsd processor
in disposable DBs. Reserve shift and99.99% LP withdrawal suppress unsafe hub
pricing without pivoting to the bad anchor; the independent-priced assets survive
until depth loss. Its external quorum/bootstrap clock is synthetic, so this does
not establish actual deployed CEX quorum or broader LP automation/recovery.

## Independent read-only production contract/access audit

[Contract inventory](production-contract-inventory.json) covers eleven deployed
contracts and six actual downloaded code blobs. Chain checksums match those
bytes; factory11711, router11712, corridor pairs11715 and CW20s10184 additionally
match existing optimized artifacts whose unchanged contract inputs are reviewed.
Router funding/settlement versions are1. Mapper11574/treasury11667 lack a matching
reviewed production artifact, and direct-wrapper capability queries were
unavailable; the protected synthetic wrapper build does not certify them.

Backend health matched original4cb573 before/after31bounded read-only RPC/audit
requests. These are explicitly independent QA reads, not quote-triggered RPC.
Pool/fee observations lack acknowledged pinned heights, so no atomic reserve
cohort is claimed. [Access feasibility](production-feasibility.json) records
masked/null authorized database credentials and absent local enrolled SSH broker
identity. No production DB connection or query was attempted. Independent atomic
production SQL/reserves, mapper/treasury artifact correspondence and physical
wallet signing stay unchecked; unavailable access cannot become fabricated PASS.

## Final related verifier and controlled browser retest

[Related source/local verification](related-final-summary.json) records all thirteen
new/related commands with zero final failures. The configured tax verifier passes
13 groups with two external-scope skips. Database isolation passes 28 groups with
zero failures and one host IPv6 skip, including actual authenticated runtime
checks, real ACL probes and first/retest of the same compiled executables. Its
first owned-cluster runtime fixture correctly failed twice because bootstrap
provided CONNECT on a second unused test database; only that owned grant was
revoked before the complete retest. Production permissions were not changed.

[Final controlled desktop/phone browser matrix](controlled-browser-final-summary.json)
passes 4,250 cases: 2,124 first, 2,124 retest and two negative controls. All 3,948
useful quotes validate, with zero provider requests and zero case failures. At
the original-input/frame 750ms budget, aggregate p50/p95/p99/max are
92.6/151.6/166.5/203.5ms. The full private receipt is retained by hash; its generated
legacy QA file is restored rather than replacing historical evidence. The nine
initial timing failures remain in their separate retained summary. These owned
fixture clocks do not establish live deployment or production custody.

## Supplemental same-quote notice retention

An independent actual Swap component regression on PR head `f44745fe` held a
validated quote with fair `1000000` and delivered net `944900` (5.51% difference).
The displayed receive stayed `0.9449` with unchanged input/generation/original
expiry while a background GET was pending, but the price notice disappeared.
The signing-stale predicate included any fetch and was incorrectly reused for
notice visibility. This reproduces a presentation defect with synthetic public
fixture inputs; it is not a reconstruction of any private report or image.

The generic display predicate now retains the notice with the current settled,
unexpired quote. Signing remains blocked during refresh and guarded mutation
entry still refuses it. New actual Swap regressions cover unchanged refresh,
exact 5%/±one raw unit transitions, immediate old-intent clearing and original
expiry. A Trade control proves its existing same-quote notice/signing behavior.
No routing arithmetic, consent thresholds, Expert/99% refusal, quote admission or
transaction pinning is relaxed. The original routing receipts above retain their
original fingerprints; supplemental final source evidence is recorded separately.

[Independent old-deployment CL8Y→USTR observation](cl8y-ustr-original-observation.json)
uses a newly selected anonymous 500-CL8Y input and 5% protection. All twelve
read-only solve/best responses validate. The first ten deliver
`40754129352501916981342` raw USTR against fair `52449818012233666974893` (22.2988%);
the last two deliver `30794523762966498277416` (41.2876%) on a different route.
Those are genuine output/route changes requiring truthful notice updates. They
do not reproduce unchanged-quote flicker, imply a missing alternative's atomic
eligibility, or reconstruct an unavailable historical amount/protection. No
private image, reporter context, wallet/actor data or production signing is
published. This observation cannot authorize suppressing a legitimate >5% notice.

[Supplemental stable notice-retention evidence](notice-retention-final-summary.json)
passes 58 affected verifier groups with zero failures and six external skips:
`verify-issue-1489`, `1483` and `1473`, each repeating stable source. The current
whole source fingerprint is `92c331987da2d6c192a22d5be88307df971f4e68dc2d09ea0e4bf7f37de37728`.
Both full frontend rounds pass 370 files/3,386 tests; the focused actual component
rounds pass 134 tests each, with unchanged source, types and lint passing.
Discovery regression rounds retain nine backend cases and now 306 frontend tests.
[Supplemental controlled browser](notice-controlled-browser-final-summary.json)
passes all 4,250 cases with zero failures/provider requests and 3948 useful outputs.
At the original 750ms clock, p50/p95/p99/max are 93.6/153.8/171.4/298.0ms.
Prior routing and prior-head CI evidence remain separately identified; this new
source requires its own genuine hosted CI and deployed acceptance. No private
report/image context is part of these receipts.

## Supplemental entry and effective market identity

[Independent deployed empty-entry baseline](empty-entry-original-baseline.json)
records an actual failure on merged `74e7ad03629832076f59614e6b37ee868fe62c9c`.
Fresh anonymous desktop and phone entries each show the unavailable-wrapping
notice at zero and five seconds with an untouched empty amount. The separately
attested served roots/backend match that revision; four read requests produce
zero writes, actors, provider reads or signing. This is independent of any private
report and retains the actual FAIL verdict.

The [Swap entry fix](../../../frontend-dapp/src/pages/SwapPage.tsx) gates wrapper
presentation on positive, representable current intent and its settled local outcome. Empty/zero input with no known operational blocker shows Enter Amount;
debounce, initial pay/receive requests and missing observations retain truthful
calculating/pricing states. Positive cached pause/treasury mismatch, unsupported
outcomes, invalid dust/precision/overflow, expired original context and unknown
budgets remain blocked. Signing/mutation validators are unchanged.

[A separate bounded baseline API observation](api-original-baseline.json) retains
its original FAIL verdict and exposes premature probe comparison:
it ended on a generation change and retrospectively compared an earlier
incomplete GET with a higher candidate learned by a later POST. That receipt does
not establish a missed three-second promotion deadline. An independent sixteen
read/five-second triage observes ten generations and several same-cohort
short-to-deep promotions in the next GET; no retained higher-known candidate
remains inferior for three seconds. Identical selected-path observation/expiry
alone cannot establish that the whole admitted market is unchanged.

A deterministic backend regression nevertheless reproduces a real identity
fault: preserving newer qualified mirrors mixed the previous opaque generation
into the next hash. Repeated identical delayed inputs with unchanged effective
views changed generation recursively, and HashMap iteration influenced the fold.
Publication now hashes the effective qualified snapshot after preservation and
fee filtering in [the canonical identity module](../../../indexer/src/api/local_quote/generation.rs). Namespace/addresses/qualification, height/time/freshness/fees,
ordered book FIFO/owner, pool companions, taxes/actors/hooks and reference/policy
inputs stay bound; the existing negative, fee/freeze/protocol, validity and wrapper
guards are unchanged. Preserved per-pair deadlines cannot renew. New controls
check collection order, original deadlines and genuine changed economic inputs.
Current follow-up source/results and genuine CI/deployed receipts remain separate
from the earlier routing and notice-retention fingerprints above.

The corrected probe pins the declared discriminator's exact original cohort,
starts at its earliest matching request start and caps the three-second window at
the original expiry. It does not reset the clock after producer changes or a
late benchmark, and a compatible post-benchmark GET is required. A prior
incomplete GET is not retroactively failed when a later POST improves; missing
original-cohort coverage remains UNVERIFIED. A proven deadline miss still fails;
fast 503 responses cannot count as useful executable socket results. This
verification correction does not relax production quote admission.

The first default-parallel verifier failed because a global fee-negative fixture
cleared the result cache while the expired-job fixture still expected its own
entry. Its held-lock assertion then poisoned the registry, causing cascading
failures. Uncaptured reproduction identifies the original missing-entry panic;
shared-global fixtures now consistently use the existing test lock while their
internal worker contention remains concurrent. The new publication fixture also
retires only its owned completed namespace. Production invalidation, caps and
mutex behavior are unchanged; the failure receipts remain retained.

A maximum current-book shape (512 pairs × 2,048 orders) took 2,396ms to hash in
debug. The original synchronous scheduled shape then failed an actual initialized
HTTP quote on a single async worker at 2,497.948ms against the unchanged 500ms
clock. Correct preparation uses two bounded CPU jobs and live-caller publication.
Earlier development first/retest controls preserved original generation/expiry
and checked positive floors/available signed attribution; 52/54 useful HTTP
responses had maximum 3ms. Final frozen-source measurements are recorded below
and in the current source receipt. The profile owns an isolated subprocess for its fixed 512-marker budget and
runs as a normal nonignored regression. Its prices/reserves/addresses are synthetic,
not production census, settlement or gas calibration. Fixture setup failures are
separate from the valid deadline RED receipt.

A controlled pending-admission regression also reproduced an old actor map
restoring approval after a known blacklist. Actor proofs now bind the completed
blacklist observation before optional discount/enrichment awaits. Commit reconciles
newer authoritative block/unblock information and hashes final bounded actor state;
a stale discount cannot replace the conservative fee after an unblock. New actor
record versions prevent recycling from accepting old proofs. Exact producer and
snapshot safety gates remain independent, and topology stays nonexecutable/advisory.
A separate deterministic local-store and actual pending-admission RED reproduced
a complete inventory reset restoring an omitted actor’s old 8% discount. Omitted
tombstones now receive the fresh reset revision, revoking their per-actor proofs;
repeated unchanged inventories keep stable versions. Both original RED logs are
retained. Final focused controls and full parallel tests are recorded in the
current-source supplemental receipt; full parallel and affected source/integration verifiers remain separate
receipts. The original actor-race and compiler/setup failure logs are retained.


## Final canonical publication source supplement

Implementation and review: [PR #1493](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1493).

[Current source receipt](core-final-summary.json) binds the modified worktree on
base `af79043096ecf80b0d85fab443f60e3f9c7c5eed` to product SHA256
`ef69138e8f0b5081e4bdb5d9a375fc1f0d78a022f13affb5422d295b7c03dca7`.
All fourteen named applicable verifiers repeat on this source: **208 PASS, zero
final FAIL, 21 explicit acceptance SKIPs**. Focused controls pass **22 tests per
round**; the actual default-parallel library passes **816 per round**, with one
existing ignored benchmark separately exercised by the applicable verifiers.
The current discovery verifier also repeats 317 frontend tests, 17 API helper
controls and two monitoring controls per round. Earlier receipts retain their
original fingerprints.

[Fresh complete controlled browser matrix](core-controlled-browser-final-summary.json)
records 4,250 cases, zero failures/provider requests and 3,948 validated useful
quotes. Original-input useful latency p50/p95/p99/maximum is
92.4/152.3/170.7/244.9ms against the unchanged 750ms budget. The synthetic maximum
publication profile retains its actual single-Tokio-worker HTTP clock and original
500ms budget; all four final focused/full rounds pass. This is source/owned-fixture
evidence, separate from final production deployment and settlement.

The first configured tax verifier retained **12 PASS, one FAIL, two SKIPs** when
the long-running owned LocalTerra chain exhausted its original **1GiB tmpfs**
before creating a test key. Only that exact-owner disposable container was reset,
with the same pinned image, original budget, no published ports or persistent data
mounts; only the new core tax code cache was invalidated. The full configured
verifier then passed **13 PASS, zero FAIL, two SKIPs**. The first failure and reset
receipt remain in the current-source evidence; production was neither reset nor
signed. Independent deployed clocks, atomic production SQL/reserves, complete
mapper/treasury artifact and capability correspondence, and physical-wallet
acceptance remain unchecked.
