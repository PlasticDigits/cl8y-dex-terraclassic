# Issue #1392 — agent review and verification checklist

Implementation: [PR #1413](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1413).

Issue [#1392](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1392),
[NET1392 invariants](../../../skills/AGENTS_CHAIN_ATTESTATION.md),
[endpoint runbook](../../runbooks/chain-attestation.md),
[indexer contract](../../indexer-invariants.md#chain-attestation-1392) and
[verification record](verification.md).

## Reproduce

1. Create an isolated git worktree, record `git rev-parse HEAD` and UTC time.
   Use the repository Rust toolchain and Python 3. No signing key or production transaction is needed. Keep Cargo build metadata
   and workspace objects private; use copies/reflinks for an optional cache seed,
   and never hardlink mutable `.fingerprint`, dep-info or workspace artifacts. Unit/shell trust tests do
   not require Postgres; focused API/cache integration checks use an **isolated**
   test database in `indexer/.env`. Provision via `make setup-indexer-postgres`
   when needed, then select a dedicated database and `TEST_DB_LOCK_FILE`. Never
   run Cargo in a root Docker bind mount.
2. Run `make verify-issue-1392`. It tests correct/wrong/missing/malformed network
   identities, unavailable/stale responses, failover, recovery, explicit overrides,
   code/hash disagreement and zero transaction preparation after failure. It
   reruns the selected suites, shell syntax and documentation links. Keep command
   output/statuses. Fix and retest every executed failure.
3. Review every direct `terrad_host_exec` and retry caller, dry-run return,
   tick and selected-LCD override. Require trust checks before financial reads,
   generation, signing, multisign and broadcast; maintain all prior custody,
   spend, quote and protocol pin rules. Probe wrong first/correct fallback, rotate
   a successful endpoint to a different chain, and change code identity mid-run.
4. Run the existing buyback and LP regression suites named by the verifier.
   Check `EXPECTED_CHAIN_ID` in indexer generated env, config rejection, startup
   checks, health schema and cache qualification. LocalTerra must be explicit.
5. Attempt read-only attestation of Columbus-5 and LocalTerra using the runbook's
   shared preflight. `make has-localterra` probes local readiness; provision with
   `make setup-cloud-localterra` when needed and available. Record observed chain,
   height/time, same-height LCD/RPC block identity and protocol-code scope. Never
   broadcast as an identity test. After deployment separately inspect `/health`
   and record its deployed revision and identity observations.

An agent may mark **overall PASS with explicit skips** when an item cannot be
completed and every executed check passed. Record exact item, attempted
prerequisite, reason and remaining evidence as **SKIP**; do not check its criterion.
Unavailable signing/deployment/device/infrastructure gates do not block source
implementation review under this task's explicit instruction. An executed failed
assertion remains **FAIL** and cannot be converted into a skip. Do not delegate
checks to the human or infer production acceptance from unit tests/read-only
probes. Optional verifier skips require a nonempty reason:
`VERIFY_ISSUE_1392_SKIP_INDEXER_REASON`, `INTEGRATION_REASON`, `TREASURY_REASON`,
`LOCAL_REASON` or `LIVE_REASON` after the same prefix.

## Issue criteria

- [ ] **AC1:** Wrong-chain/unavailable identity cannot populate discovery/price
  caches or prepare any treasury transaction. Test plausible wrong-chain contract
  JSON and assert data handlers/signing mocks were never invoked.
- [ ] **AC2:** Attest every failover/recovered endpoint. Check startup all-failed,
  mixed endpoint sets and restart/reconnect. Health/cache namespace reflects
  configured expectation and observed network, including identical code/address
  values on distinct networks.
- [ ] **AC3:** Relevant treasury scripts use shared preflight and reject mismatched
  LCD/RPC chain views. Include each retry and dry-run path; assert zero generate,
  sign, multisign and broadcast after any failed trust check.
- [ ] **AC4:** Correct Columbus-5 and explicitly configured LocalTerra retain
  working reads/dry-run paths. Do not waive old spend/quote/address/hash bounds.
- [ ] **Abuse:** Reject missing/malformed/overlarge identity, stale/future time,
  invalid height, mixed chains, block disagreement and code/hash disagreement.
  Document same-chain endpoint spoofing and initial agreeing-provider residuals.
- [ ] **Live/deployment:** Read-only probes use the deployed revision with correct
  expected-chain env; real signing/rollout acceptance requires separate evidence.

Check completed items in [verification.md](verification.md), recording test
names/commands and the scope of evidence. A checklist PASS does not close untested
live deployment criteria.
