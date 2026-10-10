# PR consolidation, 2026-10-10

PR [#1458](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1458) was the only open PR. It merged normally through `fj` at `c8b29eb0f7d07326a9745a444daff7ce76a8de4d` after available checks completed; no forced merge or branch-protection change was needed. Initial head was `53853c9fb508620a0312a634f08d9e786c39199a`. Reviewed integration head was `c923aab01589e39ae46fa2e1859d6c6b44528b1a`.

The source adds locally funded tax quotes, exact manual plans, bounded inverse sizing, incremental admission and signing anchors. There were no merge conflicts. Integration commit `c923aab0` corrects three asynchronous test waits: wait for the actual pair search result, the settled insufficient-balance blocker and the rendered token reversal. It changes test assertions only. The consolidated source matches the reviewed head.

## Verification

[Machine results](local-results.json) retain counts, revisions, exact commands and failure/skip scope. Primary verification began at **2026-10-09T23:12:25Z** on `53853c9f`; the three test-only corrections then passed the complete frontend suite and applicable related verifiers. Final-source hosted [Woodpecker pipeline 377](https://ci.cl8y.com/repos/7/pipeline/377/1) passed all six required steps for `c923aab01589e39ae46fa2e1859d6c6b44528b1a`; [filtered receipt](ci.json). Forgejo's Actions endpoint returned not found because this repository uses the external Woodpecker context; actual hosted success was verified separately.

| Verifier | PASS groups | FAIL groups | SKIP groups |
| --- | ---: | ---: | ---: |
| #1452 | 14 | 0 | 0 |
| #1398 | 7 | 0 | 0 |
| #1399 | 8 | 0 | 0 |
| #1381 | 13 | 0 | 2 |
| #1390 | 15 | 0 | 1 |
| #1422 | 10 | 0 | 1 |
| #1453 | 12 | 0 | 2 |
| #1396 | 28 | 0 | 2 |

The full frontend suite passed **3,097 tests / 358 files**. The full contract workspace passed **999 tests**. The quote verifier ran 659 library tests plus the explicit discovery benchmark, 79 socket/API tests and 339 focused frontend tests per pass. Commit-hook regression, normal commit hooks, formatting, types, lint and the four-commit secret scan passed. Existing lint/compiler warnings remain.

Initialized socket misses had maxima **12.950 / 13.872 ms**; warm/concurrent maxima were **46.193 / 45.843 ms**, with zero quote-stage provider calls. Useful baseline/later improvement timings were **2.978/6.487 ms** and **3.373/7.800 ms**. The [controlled browser receipt](browser-controlled.json) records 40 measurements, zero provider calls and a maximum **50.3 ms** across desktop/phone Swap/Trade. These are initialized synthetic loopback observations, separate from cold initialization and deployed acceptance.

Both actual 3M funded-query runs and their overhead-scoped probe transactions passed all six accounting scenarios. Funding/invoice rollback and launch-guard/refund tests used a dedicated pinned LocalTerra node. Integration binaries owned disposable QA databases; runtime/migration credentials and shared application databases were not used for QA mutation.

## Problems, skips and post-merge scope

- The first broad frontend run was **3,095 PASS / 2 FAIL**. An intermediate correction also failed two assertions (an incorrect symbol-based picker wait and a newly exposed direction-render race). These are historical executed failures, corrected and retested to **3,097 PASS / 0 FAIL**; they were not converted into skips.
- The first database verifier was interrupted while waiting for the unavailable default Compose published-loopback path. Its internal SQL worked, but bounded host probes returned no response. The final full run uses the documented explicit bridge-host skip and also records the host kernel's disabled IPv6. Internal bridge TCP, external-namespace rejection, opt-in host IPv4 confinement, authenticated runtime behavior and fresh role ACLs passed. The interrupted fixture resources were removed.
- A strict check of the old local optimized-wasm provenance receipt remains **FAIL**. Its recorded revision, three non-contract build-recipe inputs and two output hashes differ. All 170 contract inputs are unchanged, and current wasm hashes match tracked checksums; this does not establish a valid release receipt. Regenerate/review release artifacts and provenance before deployment under [#1456](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1456).
- Explicit unavailable scope: existing external native wrap stack, production execution/failover/history, independently reviewed frozen pre-fix artifacts, default Compose published-loopback positive query and host IPv6. Skips do not establish deployment or full acceptance. OpenBao session access was restored with the documented no-print certificate login; credentials never enter these receipts.

Original implementation issues #1452, #1453 and #1354 were already closed. Remaining direct-wrapper receive-floor enforcement, stateful cooldown/AutoLP modeling, uncensused tax-wallet coverage and composed inverse work are tracked in new [#1459](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1459). Deployment/live/provider/history work stays in #1456; DeFiLlama operations stay in #1449. Problems, date corrections and scope are recorded as `fj` comments on the connected issues. Unsupported runtime models remain blocked.

## Consolidation and cleanup

After merging, both remotes were fetched/pruned and local main was fast-forwarded from origin. Every local/remote branch head, including the GitLab mirror main, is contained in the consolidated main. Initial and final active stash inventories are empty. Existing historical Git replace/tag refs are retained.

All four pre-existing worktrees and their local/remote branches belong to other issue workers and are preserved, following the user instruction and the earlier consolidation ownership record. They are clean and their code is included in main. Owned temporary chain/probe resources and interrupted probe resources were removed; no shared worker service, volume or private env file was removed. This final documentation commit adds audit receipts only.
