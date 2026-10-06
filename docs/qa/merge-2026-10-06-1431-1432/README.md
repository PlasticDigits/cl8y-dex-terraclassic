# Integrated AutoLP recovery and greedy reverse admission

PRs [#1431](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1431) and
[#1432](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1432) merged through
`fj`, using openbao-tools `bao-exec`, on 2026-10-06. Their combined source tree
merged without conflicts. #1432 was first updated with the actual #1431 merge.
The initial ordinary merge returned HTTP 405 with the required hosted status
unmet. The user explicitly requested skipping CI and overriding unavailable
workflow gates. Only the required status check was temporarily disabled for
each CLI merge; original protection settings were restored and read back after
each operation. No synthetic success statuses were posted.

The resulting releases are router **1.5.0** and community-tax token **1.4.0**,
with factory **1.12.0**, pair **1.21.0** and AutoLP **1.1.0**. Four-hop greedy
reverse requests reject before external queries; supported requests retain gross
funding, original-trader economics, fresh forward sufficiency and net floors.
Explicit token-manager recovery refreshes only its bound same-pair AutoLP policy,
with exact positive reserve/share approval and atomic callback rollback.

## Local acceptance

The [machine-readable summary](summary.json) records check results and remaining
scope. The [chain summary](chain-summary.json) retains check-level outcomes and
hashes/paths of raw local receipts. These are synthetic-chain results, distinct
from production deployment and provider acceptance.

- `make verify-issue-1429`: 5 PASS, 0 FAIL, 0 SKIP, including the full combined
  contract workspace and initial/retest recovery regressions.
- `make verify-issue-1428`: 9 PASS, 0 FAIL, 0 SKIP. Each current chain matrix has
  22 successful checks and 15 fresh forward validations. Original four-hop
  greedy rejection probes consume 207,757 gas including wrapper/transaction
  overhead; that measurement is separate from smart-query gas.
- AutoLP isolated migration/recovery: 10 successful checks and 23 transaction
  receipts in each fresh first/retest fixture. The actual token 1.3→1.4 and
  AutoLP 1.0→1.1 migration, positive LP skim and complete failed-swap rollback
  are recorded.
- `make verify-issue-1398`: 7 PASS, 0 FAIL, 0 SKIP.
- `make verify-issue-1237`: 8 PASS, 0 FAIL; unchanged paid settings remain a no-op.
- `make verify-issue-1389`: 8 PASS, 0 FAIL, 1 standalone gas-command SKIP. The
  integrated baseline/current chain gas matrices were actually executed under
  #1428 rather than through this verifier's optional command variable.
- `make verify-issue-1381`: 13 PASS, 0 FAIL, 2 SKIP: native wrap stack and
  production execution. Six gross-funded scenarios and two atomic invoice flows
  pass on the owned synthetic node.
- `make verify-issue-1390`: 15 PASS, 0 FAIL, 1 production SKIP; 16 actual
  synthetic cyclic/cooldown/cap/refund/rollback guard controls pass.
- `make verify-issue-1422`: 10 PASS, 0 FAIL, 1 historical pre-FRQ baseline
  SKIP because independently reviewed pre-fix artifacts were not supplied.
  Retained current 3M first/retest fixtures each pass all six scenarios and all
  18 actual smart queries; this is distinct from the separately replayed
  router 1.4 greedy baseline and its 15M matrix.
- Strict all-target contract Clippy, Rust formatting, changed-indexer formatting,
  frontend lint/types/build, and 107 targeted receive/router frontend tests pass.
  Frontend lint retains five existing warnings.
- Local tracked/staged secret scans pass. The hosted failure on unchanged public
  Venus address literals remains an actual FAIL; hosted CI was skipped here.

The frozen router 1.4.0 matrix retains raw `status=FAIL` with
`baseline_regression=REPRODUCED`. Expected admission/work-budget errors retain
`smart_status=FAIL` alongside successful assertions of that rejection; no raw
error is converted into a successful query.

## Artifacts and merge problems

A cold digest-pinned optimizer built all 13 workspace artifacts, verified exact
source/lock/compiler/image/output provenance, and retained the separately reviewed
CW20 test dependency. All 14 release checksums match the local files. Integrating
#1431 changes token, AutoLP, launcher and pair bytes relative to #1432's standalone
checksum manifest; these four stale entries were rebuilt and corrected.
The [build receipt](build-provenance.json) records builder commit
`3ba78894`, which the final receipt merge retains in main's history. Public
provenance represents input/output maps as `path`/`sha256` arrays; reconstruct
those maps inside `metadata` to use the original provenance verifier.

Initial frontend build failed because this checkout had no WalletConnect project
ID. The corrected build uses a public synthetic build-only ID and passes; this
is not production WalletConnect enrollment. A bare git push lacked an interactive
credential source; the retry injected the OpenBao PAT through an ephemeral
askpass helper without recording its value. Earlier batch logs sharing the local
evidence directory were identified by timestamps and isolated before their names
were reused. Accepted evidence here belongs to this run.

## Follow-up and cleanup

Implementation issues #1428 and #1429 close with factual `fj` comments. Real
production migration/readback, maximum-hook/AutoLP gas, native wrap-stack setup,
trusted hosted scan-policy reconciliation, and broader populated one-to-three-hop
provider qualification remain explicit work in existing #1384/#1386/#1421.
Admission of a shorter greedy route does not establish universal gas compatibility.
No production signing, broadcast, deployment or application-database mutation
was performed.

Both remotes were fetched after the PR merges. GitLab main contributes no extra
commits. Every substantive original local/origin branch tip is in integrated main;
four intentional failing CI probes remain excluded. No stashes were found.
Original worker branches/worktrees, including their dirty historical build
snapshot, are preserved. Cleanup is limited to this task's temporary branches,
worktree, ephemeral askpass helper and two owned synthetic nodes. The final summary
records synchronization and cleanup boundaries.
