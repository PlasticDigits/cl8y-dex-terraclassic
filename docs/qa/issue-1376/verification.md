# Implementation verification — #1376

Implementation: [PR #1378](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1378).
Reviewed branch: `fix/issue-1376-receive-solver`, based on main `918461ca`.
Issue and comments were read with `bao-exec` + `fj`; no comments were present.
See [agent checklist](README.md), [E1376 invariants](../../frontend.md#solver-receive-sizing), and [AGENTS_SWAP_EXACT_OUTPUT.md](../../../skills/AGENTS_SWAP_EXACT_OUTPUT.md).

## Completed checks

| Check | Result and scope |
|-------|------------------|
| Full contract workspace, repeated after fixes | PASS: 878 tests. Workspace formatting/standard Clippy and pair all-target Clippy passed. |
| Full indexer library, serialized | PASS: 449 tests. Serialization avoids an existing shared negative-cache test collision. |
| Exact-output Postgres API | PASS: 12 tests, including real 10/100 math, competing paths, alternate after unusable candidates, fee/sender/exemption context, net taxes, freshness/fidelity, malformed modes, and fair-rate protection. |
| Existing route API / DB hybrid regressions | PASS: 29 + 8 tests. Indexer Clippy passed with existing warnings. |
| Focused receive frontend suite, repeated | PASS: 192 tests per verifier pass; covers sizing, rounding, cancellation/deadline, URL/parser, taxes, invoice compatibility, partitions and stale state. |
| Receive browser regressions | PASS: 13 cases with mocked LCD/indexer, including 10/100, rapid edits, disconnected error/retry, wrap/unwrap and URL ownership. |
| Frontend full lint and TypeScript | PASS. |
| #1256 / #713 / #678 / #512 / #1280 verifiers | PASS after correcting obsolete registry/combobox fixtures and rerunning. #1280 ran its first pass and retest. |
| Docs/schema/agent links, chrome nesting and route-solver drift | PASS. |
| Optimized Wasm build, repeated | PASS; pair artifact SHA-256 `4ad54cd3b9a5ff3d28b80bfec21800b39fee97ca9295bb524353db7590005bba`. |
| LocalTerra final artifact migration | PASS: 26 pairs on code 54 / cw2 1.19.0, with unchanged pool responses (including LP supply). [Receipts](localterra-migrations.json). Contract migration tests cover other persisted state. |
| Funded LocalTerra CW20 execution | PASS: selected two-hop route, required offer, actual CW20 send and wallet quote minimum matched the transaction. [Sanitized receipt](localterra-swap-receipt.json). |
| LocalTerra native receive guard | PASS for sizing/protection: the synthetic seed's dust quote is blocked by the measured risk gate without enabling Expert or overriding spread. Native broadcast remains unverified. |

`make verify-issue-1376` with the funded chain command completed **PASS=12 FAIL=0 SKIP=1**. The remaining verifier skip is production deployment/browser replay. The issue verifier runs contract, API and frontend checks twice, followed by lint/types/browser and optional chain/live checks. The final implementation review is **PASS with explicit skips** for production acceptance and the remaining execution matrix. Executed failures during development were corrected and rerun; they were not relabeled as skips.

## Issue criteria

- [ ] **AC1 — production 10/100 URLs:** mocked fresh-browser and real-math API regressions pass. Fresh production replay of the new implementation is unavailable until rollout.
- [x] **AC2 — competing routes / indexer-only intent:** bounded candidate comparison, alternate-path recovery and preserved receive intent verified by API/RTL/browser tests. Scope is the first five feasible fresh pool paths among 32 structural candidates.
- [x] **AC3 — old reverse boundary / explicit budgets:** realistic 32/35-call fixture, sufficient/minimal offers, orientations/scales, overflow/infeasible/exhaustion, actual router query and unchanged hybrid cap verified.
- [ ] **AC4 — complete native/tax execution matrix:** mapper/fee/tax/discount/extra-debit cases covered in unit/API tests; funded CW20 execution passed. Native execution and the entire funded tax/mapper matrix remain unverified. The native dust fixture demonstrates protected refusal, not a broadcast.
- [x] **AC5 — finite disconnected states and retry:** loading, no-route, failure, dependency/precision errors, cleared Pay, bounded retry and preserved target covered in RTL/browser/services.
- [x] **AC6 — lifecycle and stale state:** edits/clear/flip/pickers/Max/fractions/settings/wallet context/refetch and URL ownership covered; stale/placeholder/fetch/error state is checked again at mutation entry.
- [x] **AC7 — selected offer/operations/minimums:** RTL plus funded CW20 receipt verify offer-in submission against the selected route and wallet simulation; native risk gates stay active. Complete production/native/tax matrix is included in the AC4 remainder.
- [ ] **AC8 — complete live acceptance:** tests, schema/docs/playbooks, verifier, inventories and baseline probes completed; final deployed bundle/indexer/pair versions and production replay remain unavailable.

## Live baseline and release remainder

[Production inventory](migration-inventory-before.json) at Columbus-5 block **30682397** found 22 factory pairs on code **11676**, cw2 **1.18.0**. The actual public CL8Y contract metadata says **CL8Y-cb**, while the deployed ticker resolves CL8Y. The 10 target reverse succeeds; the 100 target returns the iteration-cap error, although its pool-only forward offer is sufficient. [Fresh disconnected browser baseline](browser-before.json) records published bundle paths and the offer-in solver version `global_v4`; 10 Pay resolved to `1243.234685`, while 100 Pay stayed empty. These are pre-fix evidence, not deployed-fix acceptance.

Production rollout was not performed as part of this PR delivery. Deploy the matching indexer/frontend, rebuild and migrate the inventoried production pairs to 1.19.0, then run the read-only probe and fresh-browser procedure in [README.md](README.md). Record actual revisions/code IDs/block/snapshots and repeat direct/router/invoice checks. Do not close the issue solely because this implementation review passed.

## Reproduction notes for agents

Use Node 24 and the reviewed Rust toolchain; this run pinned `RUSTUP_TOOLCHAIN=1.98.1` after provisioning unexpectedly updated global stable. Existing Postgres had a migration-checksum mismatch and very slow disk sync, so tests used isolated Postgres 16 with tmpfs. Ignored `.env.issue1376` points at its test DB; the verifier updates only `TEST_DATABASE_URL` when that file exists. Runtime uses a separate isolated database.

Dedicated browser ports require explicit local CORS origins and an indexer restart. Concurrent Playwright runs should use separate artifact directories. Mocked receive assertions allow cold Vite startup and bounded LCD delays; failed assertions still fail the verifier. Keep keys, cookies, account identities and raw signed transaction bodies out of published evidence.
