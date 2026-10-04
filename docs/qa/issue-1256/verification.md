# Implementation verification — #1256

Validated on 2026-10-04 in isolated worktree branch `fix/issue-1256-receive-input`, based on `9ee1cb44`. The PR commit contains this tested change set. See the [agent checklist](README.md), [invariants](../../frontend.md#swap-reverse-quotes), and [issue](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1256).

| Check | Result and evidence |
|---|---|
| `make verify-issue-1256` | PASS, then repeated PASS: 216 unit/RTL tests and 9 Chromium tests with 5 workers; final `PASS=5 FAIL=0 SKIP=0`. |
| `make verify-issue-713` | PASS, repeated after fixes: parser/share/Create/Trade/Swap regressions, documentation, chrome nesting; `PASS=8 FAIL=0`. Legacy live-chain browser substep skipped because this worktree has no deployed LocalTerra factory/environment. |
| `make verify-issue-678` | PASS, repeated: 41 tests and 7 verifier checks; quote-only/acquire and impact guidance preserved. |
| `make verify-issue-512` | PASS, repeated: 88 tests and 11 verifier checks; W8/W9 fee/tax accounting preserved. |
| Other existing Swap page suites | PASS, repeated: 67 tests covering amount decimals, balances, sell debit, fee registry, quote freshness, and impact behavior. |
| TypeScript `tsc -b` | PASS. |
| `make lint-frontend` | PASS with 5 existing warnings in unrelated files; no errors. |
| Independent production review | PASS after fixture corrections; connected reverse submit, pre-tax floors, stale/error protection, native fee/tax math, and silent indexer-only downgrade reviewed. |

The new browser suite uses checksum-valid synthetic contracts and mocked LCD/indexer transport, including 18/6-decimal fixtures. It exercises real browser inputs and reverse query dispatch, without a live-chain transaction. Initial failing fixture assumptions (native wrap flags, decimal scales, unsupported output URL intent) were corrected and retested; failures were not waived.

## Acceptance criteria completed

- [x] AC1: editable default/native receive, reverse pay sizing, debounced canonical URL replacement.
- [x] AC2: direct CW20 pair reverse and offer-in execution.
- [x] AC3: exact/minimal direct wrap and unwrap inverse, capped burn tax, W8/W9 rounding.
- [x] AC4: client/native multi-hop reverse operations; forward execution and hop floors retained.
- [x] AC5: silent pay-sided indexer-only fallback; no indexer exact-out or brute-force solver.
- [x] AC6: Max and all fractions restore pay direction; clear/flip, stale/error, insufficient funds, and existing safety regressions pass.
- [x] AC7: community-tax target is net; connected reverse submit uses pre-tax minimum receive without duplicating sell debit.
- [x] AC8: new verifier plus #713/#678/#512 and chrome checks pass, with the legacy #713 live browser skip disclosed above.
- [x] AC9: no added chrome layer/banner, runtime indexer API, contract, nginx, or amount-persistence changes.

## Remaining scope

Live wallet signing, live-chain swap execution, and the legacy #713 chain browser substep were not performed. Mocked connected-wallet submit assertions validate offer/minimum payload arguments; they do not prove chain execution. No production deployment or migration was requested or needed for this frontend change.

Reviewers may report **PASS with explicit skips** for truly unavailable checks, recording the reason and remaining scope as instructed in the [checklist](README.md). A skip is never a passed check; any executed failure remains FAIL.
