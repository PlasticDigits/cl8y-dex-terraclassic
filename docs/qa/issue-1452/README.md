# Issue #1452: agent review and verification

Current source coverage after #1458: [issue #1459 implementation and checklist](../issue-1459/README.md).
The #1458 scope and receipts below are historical; #1459 supersedes the direct-wrapper
protocol/signing limit, uncensused wallet admission and composed inverse rejection.
Nonzero cooldown/AutoLP remains explicitly unsupported where required public state is absent.


Implementation: [merged PR #1455](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1455). Current implementation: [follow-up PR #1458](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1458).


[#1452](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1452) requires
local, executable, liquidity-aware swap quotes and consistent signing. Read
[LQ1452-1–LQ1452-10](../../../skills/AGENTS_LOCAL_SWAP_QUOTES.md) and the
[current solver contract](../../route-solver.md#local-quote-contract-1452).
The related producer reliability work is
[#1453](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1453).

## Agent procedure

1. Use a dedicated worktree. Install frontend dependencies with the repository
   Node wrapper. Reuse compatible Cargo build artifacts if available. Database
   tests must use `indexer/.env.qa` and the per-process disposable database helper;
   never migrate, truncate or seed a shared application database.
2. Run `make verify-issue-1452`. Inspect the selection, integer output, freshness,
   network isolation, cache/worker limits and signing assertions, rather than
   counting an HTTP success or a source grep as functional proof.
3. Correct executed failures and rerun the affected checks. Record the revision,
   UTC time, exact commands, meaningful test counts, snapshot generation and
   cache counters in [RESULTS.md](RESULTS.md).
4. Run the available browser/production-like latency corpus below and separate
   read-only deployed checks. Record useful output, p50/p95/p99/max, hit/miss,
   first/later-result timing and normal transport. A 202 response is insufficient.
5. Mark each criterion PASS, FAIL or SKIP with evidence. Check off only complete
   criteria. Do not assign engineering verification to the human.

## Issue criteria

| ID | Agent verification instructions |
| --- | --- |
| AC1 | Compare CL8Y→USTR offers 0.01/0.1/1/5 and larger at identical observations; include native LUNC/USTC and wrapped forms. Compare emitted net output and actual per-hop spread for thin cLUNC versus cUSTC/UST1 corridors. Include a genuinely better shallow/book route and shuffled identities. |
| AC2 | Initialize local data, make every provider query throw/count, then quote direct/two-hop paths. Assert positive useful output and zero provider calls; include a useful corridor beyond the legacy first five/32 structural paths. |
| AC3 | Measure server request-to-useful-result and desktop/phone browser input-to-useful-result with normal transport. The current temporary target is ≤500 ms for representative initialized misses and hits, per [#1464 acceptance](../issue-1464/README.md). Preserve historical <250 ms verdicts; that tighter improvement is deferred. Report distributions/maxima and cold initialization separately. |
| AC4 | Exercise bounded deeper local search. Confirm the baseline returns first, only a meaningful eligible net improvement is published, and no UI receive reset or global-optimality claim occurs. |
| AC5 | Count provider calls across initial/deeper/reference/config/tier/refresh/preflight/confirmation work. Assert zero. Verify independent nonoverlapping ingestion cadence, deadlines/concurrency/rate limits, and that requests cannot force it. |
| AC6 | Burst identical requests, then vary exact amount/target, direction, tier/actor, spread, mode, maker cap, namespace and market/tax generations. Inspect hits/coalescing/caps and TTL expiry; forbid nonlinear amount scaling. |
| AC7 | Advance a controlled clock across 12/15/30/90 s with fixed observations; assert unchanged economics at 12/15 s and explicit expiry at 30/90 s under the immutable 20-second quote TTL. Never extend retention through a permissive book-age configuration. Change reserves/books/config, fail enrichment, poison future timestamps and expire observations; explain changes through internal evidence. |
| AC8 | Exercise typing/reversal/disconnect/Expert/confirmation refresh races. Display, fair impact, floors and signed operations must share input/context/generation. Assert positive per-hop and final floors, 30% Expert handling and the unwaivable ≥99% rule. |
| AC9 | Exercise absent/stale/unsupported/paused/frozen/blacklisted routes and dust/overflow. One accurate blocker/action must keep signing disabled. No arbitrary smaller-size advice, diagnostic chrome or live-query escape. |
| AC10 | Run focused verification and retest; review current budget/optimality documentation and crosslinks from AGENTS.md, frontend/solver docs and applicable skills. |

The [community-tax follow-up checklist](FOLLOWUP-TAX.md) covers local v1
funding arithmetic, identity/cap observations and explicit cooldown/AutoLP limits.

## Explicit skip policy

Agents may skip checks they cannot complete and mark the overall implementation
review **PASS WITH SKIPS** when all executed checks pass. Record the precise
missing prerequisite/capability, attempted command and remaining scope. A skip
does not complete that acceptance criterion or prove deployment. Executed
assertion failures remain **FAIL** and must not be hidden by adding a skip reason.
Unavailable workflow holds/gates do not block publishing reviewable source work;
runtime safety protections remain enforced.

The verifier supports `VERIFY_ISSUE_1452_SKIP_<GROUP>_REASON` for unavailable
groups: RUST, DB, FRONTEND, BROWSER, TYPES and FORMAT. The browser group runs
`scripts/qa/browser-local-quotes-1452.cjs` against a production frontend build served by a native HTTP preview
and a synthetic HTTP indexer fixture; it measures actual UI input/result and
normal loopback transport, not deployed backend or production network latency. Optional live/browser evidence is separate from local source verification.
Do not claim full issue completion when required route semantics remain unsupported.

## Deployed/browser corpus

Use synthetic fixtures and public token/contract identities, never reporter
history. Cover Swap and Trade on desktop and phone, native/wrapped entries,
forward/reverse/receive-target, pool/hybrid/tax/hook routes, typing and confirmation
races, normal request latency and provider outages. Do not broadcast production
transactions for reproduction. Read-only probes of existing deployment establish
its behavior only after confirming its revision; unpublished source cannot fix a
live probe. Preserve contract/provider reverse budgets independently of local
receive sizing.

The follow-up [mode contract and focused agent checks](FOLLOWUP-MODES.md) cover
deducting-hook inverse, exact manual POST budgets and direct zero-hop local
wrapper arithmetic. Legacy direct zero-hop signing remains explicitly blocked. The protected external
protocol and conditional source support are documented in [#1459](../issue-1459/README.md).

Current follow-up [verification receipts and issue criteria](FOLLOWUP-RESULTS.md)
record the available checks and exact remaining scope.

## Follow-up implementation review

Review the [tax invariants and supported scope](FOLLOWUP-TAX.md),
[exact modes and direct-wrapper signing limitation](FOLLOWUP-MODES.md), and
[incremental ingestion and authoritative recovery](FOLLOWUP-INGESTION.md).
The verifier now includes migrated forward, inverse, funded-tax, hybrid-book and health API regressions plus
Swap/Trade URL, indexed decimal, wallet budget, acquisition and fee fixtures.
Use the same explicit skip policy above for unavailable review steps; record
PASS WITH SKIPS only when every executed assertion passes. Runtime guards,
unimplemented semantics and failed assertions cannot become completed acceptance.
