# #1437 implementation verification

Verified in `/home/answorld/repos/worktrees/cl8y-dex-1437`, branch
`feat/issue-1437-virtual-listing-pairs`, based on
`dfb60143c432b17a92fcf9dde849257ebece4e1b`. Implementation revision:
[`7c8d6d0605a2927a694382695fe04bded8b075b3`](https://git.cl8y.com/code/cl8y-dex-terraclassic/commit/7c8d6d0605a2927a694382695fe04bded8b075b3).
Final verification completed **2026-10-07 11:36 UTC**; the following commit only
publishes this receipt.

[Issue](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1437)
· [agent checklist and explicit skip policy](README.md)
· [LV1437 playbook](../../../skills/AGENTS_LISTING_VIRTUAL_PAIRS.md)
· [contract](../../CG_CMC_COMPLIANCE.md#listing-only-native-markets-1437)
· [recovery runbook](../../runbooks/indexer-reorg-replay-dedup.md).

**Implementation: PASS. Broader verification: PASS WITH SKIPS** with the scope
and preserved external failures below. `make verify-issue-1437` finished with
**13 PASS, 0 FAIL, 0 SKIP**, including unchanged-source checks and test/retest of
the same executables. Its final SHA-256 source fingerprint is:

`3d976f8d0e26973e918bee87731aa53f2bd8909aa8689bc0e111d3c53abe279e`

The fingerprint covers all Rust source/tests, migrations, Cargo inputs and the
recovery/verifier scripts. Documentation-only receipt changes do not alter it.
The QA PostgreSQL cluster was a new synthetic container on loopback port 55437.
Every integration process created its own QA-owned disposable child database;
shared application/control databases were not migrated or cleaned.

## Issue criteria

- [x] AC1: all five published CG pair/ticker shapes, exact pins, raw 24h volume
  parity, latest indexed print, extension fields, fixed order and cache payload.
- [x] AC2: principal five versus separate fee, wrap/unwrap direction, reversed
  source asset order, human target/base prices and 6/18 USDT scaling.
- [x] AC3: missing pins/source/backing, excluded legs, zero reserves and
  uncommitted future custody/prints fail closed without breaking the list.
- [x] AC4: indexed one-level orderbook, milliseconds, historical raw-unit trade
  assignments, buy/sell, capped 500, unknown/malformed ticker behavior, zero LCD.
- [x] AC5: GT pair wrapper/leg identities, native CoinGecko ids and pinned USDT
  override; wrapped legs do not inherit native ids.
- [x] AC6: real tx/sender/block linkage, translated postevent reserves, factory
  event preservation, unique combined event indexes, exact 5000/5001 factory cap.
- [x] AC7: factory/economic tables and retail, CMC, DeFiLlama and routing remain
  isolated; opaque-id variants reject before SQL/LCD/discovery.
- [x] AC8: spoofed symbols/USDT cannot replace pinned legs; real CG tickers win
  collisions, including a previously cached factory-map miss.
- [x] AC9: existing listing/indexer/frontend unit regressions pass under the
  explicit #655 optional-browser exclusion below. Full live browser acceptance
  is not claimed.
- [x] Documentation: invariants, agent skills, review/skip instructions, recovery
  and the prepared GT remark are crosslinked with implementation and issue.
- [ ] Optional #655 live browser pool-table acceptance: original run failed on
  the existing Vite import from `public/chains/chainlist.json`; remains unverified.
- [ ] Production deployment, historical live replay and GT form submission:
  outside this issue's stated scope; none performed.

## Commands and results

| Command / check | Result |
|---|---|
| `make verify-issue-1437` | PASS: 13 checks; API 10/10, principal integration 3/3 and library 575/575, each twice using the same executable. The one normally ignored manual discovery benchmark ran in #1394/#1396 separately. Recovery DSN/output privacy passed twice; source boundaries and formatting passed. |
| `make verify-issue-685` | PASS: CG/CMC/GT integration, shared exclusions/spread and docs/source checks. |
| `make verify-issue-646` | PASS: GT routes, shapes, asset/pair/events and unit checks. |
| `make verify-issue-684` | PASS: stamped postevent reserves and legacy backfill behavior. |
| `make verify-issue-694` | PASS: event caps, strict rate-limit routes, security/progress integration and frontend tests. |
| `make verify-issue-631` | PASS: DeFiLlama SQL isolation, exclusions, daily API and Node adapter tests. |
| `VERIFY_ISSUE_655_SKIP_E2E=1 make verify-issue-655` | PASS for available checks (8 reported steps); optional browser step is SKIP, not evidence of acceptance. Pair-TVL fixture tests passed 10/10 and 10/10 independently. |
| `make verify-issue-1395` | PASS after fixing recovery's explicit-DSN overwrite; library and raw-event integrity test/retest plus all selected ingestion/fee/volume/limit suites passed. |
| `VERIFY_ISSUE_1396_SKIP_BRIDGE_HOST_REASON=... make verify-issue-1396` | PASS WITH SKIPS: 28 PASS, 0 FAIL, 3 reported skips. Runtime-login fixture was subsequently supplied privately and passed twice in its confined child DB; other socket/ACL/migration checks passed twice. |
| `make verify-issue-1394` | PASS after correcting rustfmt to use Cargo's declared 2021 edition: 23 PASS, 0 FAIL. Includes repeated default/high concurrency, benchmark and Postgres test/retest. |
| `VERIFY_ISSUE_1392_SKIP_LOCAL_REASON=... VERIFY_ISSUE_1392_SKIP_LIVE_REASON=... make verify-issue-1392` | PASS WITH SKIPS: 10 PASS, 0 FAIL, 2 SKIP. Library, check-all-tests, synthetic ingestion/attestation/treasury and docs passed first/retest; deployed local/live endpoint reads remain unverified after explicit attempts. |
| `scripts/ci/gitleaks-scan-tracked.sh --verbose --exit-code 1` | PASS after staging the complete implementation: no leaks found. Normal commit hooks also passed contract formatting/clippy, frontend TypeScript and staged secret scanning. |

Independent agents implemented ingestion and projection separately, reviewed the
assembled changes and ran isolated integration tests. Review caught the fee SQL
type mismatch, middleware LCD calls before virtual lookup, failed-transaction
projection, empty-log fallback and committed-checkpoint visibility. Each was fixed
and the settled source passed the final verifier. Earlier failures from those
iterations are superseded by passing fresh builds, not reclassified as skips.

The #655 legacy fixtures used uncertified `oracle_prices` averages and hub marks
without source time/quality. They now seed the same fresh two-source observations
and `bounded_spot` timestamps required by current production readers. Production
pricing and test assertions were not weakened.

Recovery now retains an explicit `DATABASE_URL` through setup defaults, selects
host `psql` for that capability, omits credentials from output, and deletes both
projection tables during reorg cleanup. Actual #1395 recovery ran against only
its process-owned child database; a separate subprocess test verifies DSN
selection and redaction without any database contact.

## Unavailable checks and preserved scope

- **#655 optional browser:** attempted rather than assumed unavailable. Existing
  `terraExplorer.ts` imports JSON from Vite's `public` directory, which the dev
  server rejects; the pool-table assertions consequently failed. That executed
  browser failure remains recorded. The available-check rerun explicitly
  excludes it; no browser PASS or blanket all-tests-green claim is made. A
  separate frontend repair and browser rerun is a relevant follow-up.
- **#1396 default Compose bridge loopback:** dedicated published-host TCP probe
  timed out while an internal `SELECT 1` succeeded. The long unavailable probe
  was stopped and its disposable stack removed. The rerun used the supported
  explicit bridge-host skip; internal bridge access, external denial, host-mode
  IPv4 and fresh role ACL checks still ran. Host IPv6 is disabled by the kernel.
- **Runtime login fixture:** its first attempt correctly failed because the
  synthetic runtime also had CONNECT on the unused synthetic application DB.
  The fixture was confined by temporarily removing that grant, passed twice,
  and the original synthetic application grant was restored. This changed no
  shared or production role.
- **Read-only live LCD:** configured PublicNode and Hexxagon returned HTTP errors
  and the Terra endpoint was unavailable. No fresh attested live read is claimed.
  `make has-localterra` also confirmed no deployed local chain; synthetic HTTP
  attestation, height-header, stale/wrong-chain and recovery tests ran instead.
- **OpenBao wrapper:** invoked
  `/home/answorld/repos/openbao-tools/workstation/bao-exec --only FORGEJO_PLASTICDIGTS_PAT -- fj issue view 1437`.
  It failed because the workstation vault VPN route was unavailable; noninteractive
  sudo was unavailable for route inspection. Existing authenticated `fj`/git
  access read the issue (zero comments) and publishes the PR. No credential,
  authentication policy or recovery boundary was changed.

Per the task's explicit override and the checklist skip policy, unavailable
external gates do not hold the implemented, locally verified PR. The failing
browser and unavailable live/socket results remain unverified; they are not
converted to deployment or crawler acceptance evidence.

## Projection interpretation and rollout

Projection dedup is per **indexed source event** (`swap_index` or mapper
`source_ordinal`), consistent with one trade per real source fill. Multiple genuine
fills within a single transaction retain distinct prints; replaying a source
event never creates another copy. The issue's shorthand “one tx” is not treated
as permission to discard distinct genuine fills or invent a netted fill. A strict
one-print-per-transaction aggregation is not implemented or claimed.

Historical mapper principal needs archive bank reads at the source and predecessor
heights with matching height headers. Missing/pruned or unreconciled custody
omits the affected listing projection; historical fee rows cannot reconstruct
principal. A later deployment uses the separate privileged migrator job and
normal indexer startup; this PR neither deploys nor submits the listing form.
