# Issue #1452 implementation evidence

This record covers the `fix/issue-1452-local-quotes` worktree based on
`b26b762c4a870251d780715eac6eea4e2db5c14e`, investigated 2026-10-09 UTC.
Issue and comments were read with `bao-exec --only FORGEJO_PLASTICDIGTS_PAT -- fj`;
the issue had zero comments. No reporter history, production pool/book mutation
or production signing was used. Final source revision and command results are
recorded below after verification.

## Supported implementation

The current request path uses initialized immutable observations maintained by
scheduled workers. It emits local arithmetic quotes with positive floors and
exact amount/target, actor, constraint and generation identity. Requests do not
call chain identity verification, router/pair simulation, tax/config/tier discovery
or remote references. Unknown required semantics fail explicitly. The legacy
request-time LCD implementation remains a historical reference only.

Pool routes use checked wide integer constant-product math and fee rounding.
The local book model retains Policy A: only the first hop may declare a split,
with subsequent offers propagated through the emitted plan. Cached supported
tax/burn ask hooks deduct against the original output with per-hook floors;
non-deducting supported LP hook pricing remains separate from callback success.
Native wrapper observations bind treasury/mapper/mapping/fees/pause/rate limits;
input mint applies mapper fee once without BankMsg burn tax. Native output needs
actual observed burn-tax rate/cap and router binding, preserving the pre-bank-tax
router minimum basis. Receive sizing remains offer-in execution and unsupported
inverse models fail explicitly.

## Acceptance checklist

A checked box represents the complete listed criterion; narrower passing evidence
is described beside unchecked criteria rather than claiming full acceptance.

- [ ] AC1: full CL8Y/native corpus and all admitted delivered-net semantics.
- [x] AC2: eligible direct/two-hop search scores cached data locally with zero RPC; unsupported models fail explicitly. Source/provider-stub proof is limited to the supported implementation above.
- [ ] AC3: strictly <250 ms server and production-like browser corpus.
- [x] AC4: bounded deterministic later improvement returns after a useful baseline; no global-optimum claim or receive reset.
- [ ] AC5: zero network throughout request flow and bounded ingestion evidence.
- [ ] AC6: cache reuse/coalescing, exact inputs/targets, namespace, spread, actor, reserve/reference/book/hook/pin/freeze invalidation are implemented and locally tested. Complete tier/tax and large-graph abuse coverage remains unfinished.
- [ ] AC7: controlled 12/15/30/90-second observation ages, stable output, cache reuse, changed reserves/freeze and reference outage/recovery pass locally. The complete populated-book/tax refresh corpus remains unverified.
- [ ] AC8: complete display/preflight/confirmation/signing generation parity.
- [ ] AC9: accurate unavailable blocker and no unsupported size guidance.
- [x] AC10: focused verifier, retest, bounded-work documentation and source/agent crosslinks.

## Explicit unverified scope

- SKIP: deployed frontend/indexer validation and production-like normal-transport
  latency. This source branch is not a deployed service; existing production probes
  cannot establish its performance. Production deployment was not requested.
- SKIP: physical phone/wallet signing. Browser/unit fixtures are independent source
  evidence and cannot prove device signing or actual chain settlement. Controlled
  browser runs use disconnected wallets; ancillary wallet balance/account and
  actual transaction broadcast network reads remain outside that measured corpus.
- Known indexed actor blacklist/discount contexts are cached; unknown actor
  contexts use conservative full-fee estimates. They are not fresh wallet-specific
  blacklist/launch/exemption approval. Community-tax forward routing, deducting-hook
  inverse, explicit manual-split POST and direct zero-hop wrapper quotes remain
  unsupported. Existing execution controls cannot be waived to quote them.
- The capped admission pass publishes after completing configuration checks. At
  the largest graph/actor population, provider pacing may exceed its 180 s
  deadline and prevent initialization; incremental admission remains unfinished.
  The representative initialized fixture does not establish that large-factory
  liveness or production readiness.
- Independent reserve observation/provider failover and historical liquidity
  repair remain [#1453](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1453).
  A reserve refresh consumes original indexed observation times; this PR does not
  repair historical missing observations or claim production feed recovery.

## Verification receipts

Initial socket verification caught a real request-triggered middleware chain
probe; it was treated as FAIL and corrected before retesting. Initial review
also corrected the L9 denominator and constrained inverse-bracket failure. These
findings are regression inputs, not successful first-run evidence.

Final source verification was run twice by:

```bash
CARGO_TARGET_DIR=/home/answorld/repos/cl8y-dex-terraclassic/indexer/target make verify-issue-1452
```

Started **2026-10-09T09:24:38Z**, completed by **09:31:58Z**. The tested working
source was based on `b26b762c4a870251d780715eac6eea4e2db5c14e`; source fingerprint
`ab18adbf41b59e4b37adb899619811b5d9b947a8ee6706dab43a906525f890f9`
remained unchanged across build/test/retest. **14 groups PASS, 0 FAIL, 0 SKIP**
within the local verifier scope. Each run passed **604 library tests**; its one
ignored manual discovery-cache benchmark was explicitly executed and passed.
Each run also passed **3 isolated socket tests**, **181 frontend tests in 14
suites**, and **40 controlled browser measurements**. Types, Rust formatting,
shell syntax, whitespace and invariant crosslinks passed.

| Socket measurement | First run | Retest |
| --- | --- | --- |
| Cold configuration/wrapper initialization (excluded) | 8,131.607 ms | 8,120.918 ms |
| 67 initialized response-cache misses: p50 / p95 / p99 / max | 2.338 / 2.689 / 2.793 / 14.811 ms | 2.241 / 2.482 / 3.095 / 12.619 ms |
| 92 warm/concurrent requests: p50 / p95 / p99 / max | 0.822 / 57.280 / 57.315 / 57.324 ms | 0.798 / 48.656 / 48.733 / 48.831 ms |
| Useful baseline / later improvement | 2.326 / 6.075 ms | 2.312 / 5.638 ms |

Socket fixtures use block **1452**, eight competing mixed-decimal corridors,
native input fee-rounding collisions, hooks, independent reference marks, actor
and spread identity, freeze/unfreeze, changed reserves and later three-hop
improvement. First baseline generation was
`31ea155f8b9b25dd5a9942fdf359815459b1100fefdb9108549851a6ba47c4a1`,
with delivered raw output `9920278247176726816`. Providers returned errors for
all quote-stage requests; their observed call count remained **zero**. Cache
counters in the corpus were **92 hits**, **67 background starts**, **0 active
jobs after settlement**, **67 amount-sensitive entries**.

Controlled full **Google Chrome 151.0.7922.137** measured actual input events to
useful DOM output over native local HTTP using a production build and a
synthetic indexer. Each surface/viewport owns a browser process and measures
initialization separately; no sample filtering or relaxed threshold is used.
Desktop/phone Swap and Trade each have ten samples per run. First maxima were
**62.7 / 36.0 / 52.2 / 37.3 ms**, retest maxima
**51.9 / 35.1 / 54.5 / 38.4 ms** (desktop Swap/Trade, phone Swap/Trade).
All runs had **zero provider requests, zero quote/initialization chain attempts,
zero page errors**. These controlled results do not complete AC3's deployed
production-reference corpus. [Latest receipt](browser-controlled.json),
[isolated first](browser-controlled-isolated-first.json) and
[isolated retest](browser-controlled-isolated-retest.json) retain full timings,
Chrome flags, HTTP and resource timing.

Additional safety verification:

```bash
CARGO_TARGET_DIR=/home/answorld/repos/cl8y-dex-terraclassic/smartcontracts/target cargo test --manifest-path smartcontracts/Cargo.toml --workspace -- --test-threads=1
```

**999 contract tests PASS**, zero failures/ignored tests; contract Clippy also
passed with the repository hook's existing lint arguments. No contract source
was modified.

Earlier failures are preserved rather than called passes: request-time chain
middleware, the wrapper refactor's missing Rust type annotation, browser port
collisions, and timing pauses in the shared headless browser lifecycle. The
browser harness now binds OS-assigned ports, serves a production build over
native HTTP, uses the installed full Chrome and isolates case lifecycles.
[The failed transport receipt](browser-controlled-failed-transport.json),
[diagnostic receipt](browser-controlled-diagnostic.json) and
[failed shared Chrome retest](browser-controlled-chrome-retest.json) remain
available. Source code also now retains known wallet blocks through unavailable
fee/blacklist data and atomically rejects reserve publications from older policy
epochs; dedicated regressions pass. Overall PASS WITH SKIPS permits unavailable review tasks only;
it never converts an executed failure or an incomplete acceptance criterion to
PASS. Follow the [agent procedure](README.md) for future verification.
