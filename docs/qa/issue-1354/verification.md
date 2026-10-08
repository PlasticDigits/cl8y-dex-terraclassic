# Verification result: FAIL, keep #1354 open

Verified on 2026-10-08 UTC from worktree `verify/issue-1354-defillama`, source
revision `48f01fd1` (full revision in the [public receipt](public-reconciliation.json)).
The serving indexer reports `4572e8779801c5c90b63ec95a0c7d5c38ce50bbe`.
This change updates QA/docs and the upstream volume copy; it does not deploy or
repair indexer accounting. Issue and its comment were read using openbao-tools
`workstation/bao-exec` with `fj`. #631/#687 remain closed.

## Completed UTC window, 2026-08-26 through 2026-09-24

| Observation | Current result |
|-------------|----------------|
| Stored Llama volume | $53,178; all ten formerly missing dates now present |
| Broader protocol priced volume | About $53,177.23; matching daily trade counts permit the narrow upstream fallback |
| Live Llama feed | Ten dates still null, no priced/unpriced completeness counts |
| Stored Llama fees | $746.87 |
| Current daily-feed fees | $816.735930564854092361 |
| Broader protocol fees | About $897.47; do not attribute its additional amount without source/pair SQL |
| Latest Llama TVL token basket | LUNC and USTC only; current coins request omits UST1/USTR/CL8Y |

These figures are current observations, not a replay of the issue's September 25
snapshot. Llama volume recovery is real progress: upstream
[#9907](https://github.com/DefiLlama/dimension-adapters/pull/9907) merged on October 4,
commit `3d8f4ca7f1939a784b3e74563cb938071b900fd9`. Both copied volume files were
compared byte-for-byte with that pinned revision; [hash receipt](upstream-volume.json).
The fallback requires matching UTC identity/catalog and exact trade counts,
rejects malformed/duplicate/unpriced data, and cannot recover beyond 90-day
retention. The fee/TVL adapters still differ from the local testable stand-ins.

September 24 remains `volume_usd: null`, 28 trades, versus the protocol series'
$558.461005050729077642. Llama now stores $558. Same-day fees are
$2.008622408232357884 versus $2.01 stored. Other historical fees remain divergent:
for example September 17 is about $166.95 feed versus $115 stored.
The August 17 volume is still absent upstream despite a positive $716.51 feed;
the recovered 30-day window does not establish complete start-to-present history.

## Agent checks

| Command/check | Outcome |
|---------------|---------|
| `make verify-issue-1354` | FAIL on public accounting findings; local copy/tests/cross-links recorded separately |
| `node --test scripts/defillama/dexs/volume.test.js` | PASS, 7 boundary tests |
| `python3 scripts/qa/test-reconcile-defillama-1354.py` | PASS, 4 null/precision/unavailability/completeness tests |
| `make verify-issue-631` | PASS, 5/5; 4 Rust unit tests, 11 isolated integration tests, Node helpers |
| `make verify-issue-687` | FAIL, 6/7; its own 4 unit/11 integration tests and mapper pass; related #683 fails |
| Related `VERIFY_ISSUE_683_SKIP_RELATED=1 make verify-issue-683` (called by #687) | FAIL, 5/6: 6/8 economic-fee integration tests pass, 2 fail |
| `bash scripts/qa/reproduce-defillama-1354.sh` | FAIL, 0/2 desired accounting acceptance probes |
| Read-only fee SQL on the owned synthetic fixture | PASS: executes/rolls back; detects $1.50 mixed-transaction fee loss |
| `make verify-issue-1396` | FAIL: 26 PASS, 2 FAIL, 3 explicit SKIP; both failures are the same Swagger fixture, first/retest |

The [acceptance probe source](acceptance-probes.rs) is appended temporarily to the
existing integration harness, filtered to its two tests, then removed. Each process
creates its own marked QA child. [Failure log](acceptance-probes.log):
partial positive $558.461005 plus an unpriced swap returns null; a $1.50
ordinary pair fee sharing a transaction with a gem swap rolls up to zero.
[SQL fixture](sql-fixture.txt) independently reports current-rule zero versus
verified ordinary-pair $1.50, with no unresolved scope in that synthetic row.

The [related failures](related-gate-failures.log) are
`cl8y_ask_fee_stamps_usd_and_rollup` (missing CL8Y mark) and
`null_only_backfill_does_not_rewrite_stamp` (filled 0, expected 1).
Both are repeatable fixture/test failures, not infrastructure skips. Determine
whether the fixture needs the newer qualified hub prerequisites or pricing has
regressed before claiming #683 compatibility. Its existing tests were not weakened.

[Database isolation results](database-isolation-results.log) include all 578
library tests and representative isolated integration binaries, first/retest,
real ACL denials, launcher capability stripping and network confinement.
[Failure](database-isolation-failure.log): `security::swagger_ui_available`
expects `/swagger-ui/` HTTP 200 but gets 404 on both attempts. This is an executed
failure, not a skip. The initial run was interrupted while the unavailable
bridge-published loopback handshake waited; the rerun narrowly skipped that
positive host query and disabled host IPv6. Internal TCP, host-mode IPv4 and
external-namespace denial probes passed. The third skip is an uninjected optional
authenticated-runtime fixture; separate real role ACL probes passed.

Source review confirms `DAILY_LOOKBACK_DAYS = 8`, no explicit DeFiLlama history
backfill CLI/dirty-day queue, tx-hash-wide gem fee exclusion, and `FeeSource::ALL`
still containing seven entries. #1209/#1210 are closed in Forgejo but comments
and source explicitly retain unimplemented pair_creation/SKU/settings ingest.
The public `/protocol/fees` still exposes only those seven sources.

## Explicit unavailable evidence

- **SKIP: production SQL reconciliation.** The old Coolify token returns 401;
  the scoped wire token can read app/server metadata. Default noninteractive SSH
  to its actual server reaches authentication but returns `Permission denied
  (publickey)`. Three additional existing local SSH identities also fail
  noninteractive authentication. No database secret was exported and no maintenance job/deployment
  was run. The ready SQL needs existing authorized read-only access; agent work
  resumes there when custody/access is available.
- **SKIP: same-block TVL acceptance.** Public LCD factory/block probes returned
  403. Last Llama token data and current coins coverage are recorded, but they
  are not a pinned reserve/price comparison or an exact TVL target.
- **SKIP: historical repair/reload, price onboarding, monitoring scheduling.**
  No such actions were performed. Read-only diagnostics are available; accounting
  fixes must precede historical writes and fee reload requests.

These skips use the user's explicit permission and never convert executed
accounting failures into PASS. See the [agent checklist and short remaining list](README.md),
[DL1354 invariants](../../../skills/AGENTS_DEFILLAMA.md#live-reconciliation-1354),
[methodology](../../DEFILLAMA.md) and [testing catalog](../../testing.md).
