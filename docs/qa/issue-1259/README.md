# #1259 — number display verification

Issue and comment: [#1259](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1259). The comment confirms Compact first-visit default and Full grouped opt-in. Invariants: [**Q1259-1–Q1259-10**](../../frontend.md#number-display-preference). Agent playbook: [`AGENTS_FRONTEND_NUMBER_DISPLAY.md`](../../../skills/AGENTS_FRONTEND_NUMBER_DISPLAY.md).

## Agent execution and result policy

Run everything available before recording a skip. Agents may skip unavailable checklist items and mark the overall verification **PASS with SKIPs** if all runnable checks pass. There are no holds or deployment / wallet gates for this issue. Record the reason and unverified scope for each SKIP. A real failing check is FAIL until corrected and retested; it cannot be relabeled SKIP. Do not check off an acceptance item based only on an intended implementation or a test that did not run.

Use this evidence format in the review / PR: `ACn — PASS | FAIL | SKIP — command or observation; skipped reason if applicable`. Record the tested commit, environment, and commands. Distinguish mocked browser rendering from actual chain execution and production visual QA.

## Commands

```bash
make verify-issue-1259
make test-frontend
make lint-frontend
# Optional isolated real-UI smoke; mocked LCD reads, no chain or wallet required.
VERIFY_ISSUE_1259_BROWSER=1 make verify-issue-1259
# Rerun after corrections.
make verify-issue-1259
```

The verifier uses five Vitest workers and exercises formatter / preference tables, Protocol / Charts / tape consumers, price and raw input exceptions, Swap Settings, and signing summaries. The browser smoke runs with `PLAYWRIGHT_SKIP_CHAIN=1`, synthetic #1256 factory pins, and mocked LCD responses; no fixture DOM is substituted for production Settings. Browser screenshots are attached to Playwright results for the selected theme / viewport. Browser checks are SKIP unless explicitly enabled; missing runnable dependencies should be provisioned where feasible.

Existing #522 / #553 / #557 / #564 / #565 verifiers include indexer / Postgres work unrelated to this UI preference. Run them when their environment is available, record any unavailable scope, and use the targeted existing frontend suites included here for display regressions. No contract migration, indexer schema, or Coolify environment change is required.

## Acceptance checklist

Checked rows have passing automated coverage and source review in this implementation. Deployed multi-page / real-wallet extensions within those rows remain SKIP as recorded below; a checked row does not claim an unexecuted live check.

| Done | Criterion | Agent instructions and evidence |
| --- | --- | --- |
| [x] | AC1 — Compact default | Run formatAmount and preference tables with an empty key. Check current examples `13.23K` / `1.235M`; first-visit Compact chip pressed. |
| [x] | AC2 — Full across chrome | Run Protocol / Charts / tape / formatter tests; in real UI select Full, check grouped balance / receive display, Charts token 24h Vol after 18-decimal humanize, Pool Vol USD, Protocol USD, tape amounts, and Tiers thresholds. Typeable Receive stays a plain draft (AC4). Note any surface requiring unavailable data. |
| [x] | AC3 — Price exception | Run formatPairPrice / tape price / Charts OHLC tests in both modes; check no K/M/B/T ratio suffix and unchanged ratio scale. |
| [x] | AC4 — Inputs / raw payload | Run Swap Settings input test and amount round trips. Type `1000.5` under Full, toggle both modes, and compare draft + `toRawAmount` output. Other money / slippage / deadline inputs remain plain decimal. |
| [x] | AC5 — Signing chrome | Run SwapPreSubmitSummary, LimitOrderPreSubmitSummary, and PoolPreSubmitSummary tests with Compact selected. Check grouped full amounts and exact raw-to-human fractional digits. Wallet / chain execution, if unavailable, is SKIP separately. |
| [x] | AC6 — Storage / URLs | Run hostile / missing / denied storage tests, browser reload after Full, query opt-out, and canonical swap Share tests. Only legal storage values persist; no format / compact / locale URL key is added. |
| [x] | AC7 — Decimals / caps | Run Charts / formatAmount cap tables: unresolved, negative, noninteger, >18 decimals, HTML, noninteger raw, and 78-digit volume all return `—` where guarded. The 24-character cap remains 24. |
| [x] | AC8 — Chrome | Run `python3 scripts/check_chrome_nesting.py`; inspect existing `#swap-slippage-settings` for labeled chips / preview without added cards. Check 375px / 1280px, light / dark, visible pressed and keyboard focus, 44px targets. |
| [x] | AC9 — Execution protections | Run existing Swap / canonical URL regressions. Review diff for changes to expert / hybrid / slippage / simulate / execute behavior; optional real quote / wallet test must retain offer and min received across mode changes. |
| [x] | AC10 — Docs / tests | Run verifier docs checks and full frontend suite. Review crosslinks from frontend guide, testing, docs index, AGENTS, and chrome playbook. Retest corrected checks. |

## Attack / abuse checklist

| Done | Vector | Agent instructions |
| --- | --- | --- |
| [x] | A1 hostile storage | Try `<img src=x onerror=alert(1)>`, `javascript:alert(1)`, mixed case and unknown enum values. Expect Compact; no HTML execution. |
| [x] | A2 query preferences | Open `/?format=full&compact=0&locale=de-DE` without a legal stored preference. Expect Compact; format keys do not control Settings or enter the canonical Share URL. |
| [x] | A3 oversized volume | Feed a 78-digit raw string and valid decimals to guarded pair volume. Expect `—`, unchanged cap, and responsive rendering. |
| [x] | A4 comma draft | Try `1,000.5` in You Pay. Expect existing draft validation / sanitization, no throw, and no formatted amount sent to BigInt / execution. |
| [x] | A5 mode during quote | Capture offer / min received raw before and after toggling. Expect identical raw values and no preference-triggered quote refetch. Use deterministic unit evidence; mark real wallet signing SKIP if unavailable. |
| [x] | A6 Share | Copy the Swap Share URL after Full. Expect canonical token / exact amount keys only; display preference remains local. |
| [x] | A7 hidden gems | Review production discovery filtering and existing tests. `?showGems=1` / display preference must not reveal hidden ids or alter exclusions. |
| [x] | A8 signing always Full | With Compact selected, open every relevant summary fixture; check grouped amounts and no compact amount suffix. |

## Optional live visual walk

Start the existing configured frontend with `make dev`. At 375px and 1280px in both themes, open Settings, select Full, capture a screenshot, toggle back to Compact, and reload after Full. Visit Charts, Pool, Protocol, Trade tape and Tiers with known data. Compare prices in both modes, then enter `1000.5` without commas and compare quotes. Check Expert Mode / >30% route slippage protection on a suitable existing fixture. Do not broadcast only to prove a display preference. Record absent data, wallet, or deployment access as SKIP, with overall PASS with SKIPs permitted.

## Evidence record

Tested branch: `fix/issue-1259-number-display` (implementation worktree; the PR commit list identifies the final revision). All criteria above have passing runnable coverage. Overall **PASS with SKIPs**: live deployed multi-page data / real wallet signing / production WalletConnect remain unverified.

| Result | Check | Executed evidence |
| --- | --- | --- |
| PASS | AC1–AC10 targeted regression | Final `make verify-issue-1259`: 20 files / 316 tests; formatter, store, consumers, signing, raw-input / price exceptions, required gas-balance digits, Tiers memo / table consistency, Protocol, and Settings. Chrome guard and invariant crosslinks passed. |
| PASS | AC1 / AC4 / AC6 / AC8 browser | `VERIFY_ISSUE_1259_BROWSER=1 make verify-issue-1259`: 5 browser tests; real Settings at 375px / 1280px in dark / light, Compact default, Full grouping / reload, unchanged Pay draft / URL, hostile storage / query ignored, 44px targets. Earlier targeted suite in the same run: 17 files / 292 tests. Later gas-message additions were covered by the final 316-test run. |
| PASS | Full frontend retest | `npm test -- --run --maxWorkers=5`: 347 files / 2,990 tests. Reverse-input assertions now expect unchanged plain decimal drafts (`2`, `6`); the pre-existing ALPHA pane test was corrected to expect the absent USD inversion pill, matching its existing behavior. |
| PASS | Lint / production build | Lint: 0 errors / 5 existing warnings. Production build passed with synthetic `VITE_WC_PROJECT_ID=00000000000000000000000000000000`; this proves the build, not real WalletConnect connectivity. |
| PASS | Existing scale / volume regressions | `make verify-issue-564` (3 checks / 141 tests), `make verify-issue-565` (3 checks / 78 tests), `make verify-issue-522` (5 checks), `make verify-issue-553` (4 checks), `make verify-issue-557` (5 checks). Includes host Cargo unit tests and serialized Postgres integrations. |
| PASS | Visual Settings evidence | Four screenshots saved by Playwright under `frontend-dapp/test-results/number-display-1259-*/settings-full-{theme}-{width}.png` and attached to test results. Inspected dark 375px: Full selected / grouped fixture legible inside existing Settings, without new cards. Regenerate with the browser command; generated artifacts are ignored by git. |
| SKIP | Live multi-page / wallet / production | No live deployed Charts / Pool / Protocol / tape / Tiers data walk or physical wallet signing was performed. Deterministic unit / component tests and mocked-read browser tests passed. No contract / indexer / deployment change was needed for this display preference. |

Retest notes: initial browser expectations omitted the existing `Example:` label; corrected and retested. Cold Vite imports under concurrent full-suite / Cargo load exceeded initial readiness timeouts; readiness now allows the existing 90-second UI convention, and all five tests passed. Initial Postgres regressions hit pre-existing shared migration checksum `VersionMismatch(20260916120000)` before test execution. Creating isolated `dex_indexer_test_1259` and updating the ignored worktree `indexer/.env` resolved that environment problem; all three verifiers passed without changing shared data or migration checksums. Real failures were corrected / retested, not labeled SKIP.
