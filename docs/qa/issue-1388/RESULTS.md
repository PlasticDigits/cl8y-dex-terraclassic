# Issue #1388 verification results

**PASS with explicit skips** for implementation acceptance. Production deployment
is unverified and remains separate under [#1384](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1384).
[Agent checklist](README.md), [invariants](../../../skills/AGENTS_ROUTER_TRUST_LIFECYCLE.md),
[recovery runbook](../../runbooks/router-trust-1388.md).

Tested on 2026-10-05 UTC in isolated worktree `fix/issue-1388-router-trust`, based
on `9944ada1`. Final recovery source Git blob:
`efd79835917e328d1fbaeee816ce937c6cb105e3`; discovery source blob:
`1efb96e82fcc26fc018a586ca67a3774d22f540e`. The PR commit identifies the complete
reviewed tree. Final independent source/security review passed.

## Completed checks

| Check | Outcome |
|---|---|
| `make verify-issue-1388` | Final source checks and retest: **PASS=12 FAIL=0 SKIP=1**. Its chain evidence group validates the separately executed, recorded SDK receipts below. The skip is production execution. |
| `make verify-issue-1381` | Funding/solver/frontend regression and retest: **PASS=13 FAIL=0 SKIP=1**, including actual migration and native flows. After final recovery changes, the full contract workspace, focused verification and both on-chain recovery rehearsals were repeated. |
| Contract workspace | **890 tests passed**; includes **113 token tests**, foreign adoption, legacy direct-pair/SendFrom, directory/guard regressions, replay, custody, factory-governance recovery with opaque admin, counter exhaustion and migration preservation. |
| Trading discovery | **8 focused tests passed and retested**; full indexer library **475 passed**. Ordinary/legacy/funded/unsupported cutover, failed live identity, migration during discovery and hash/router attestation covered. |
| Postgres APIs | **55 passed and retested** across exact output (12), route solve (29), DB hybrid (8), route-tax funding (6), using a dedicated test database. |
| Frontend | **268 tests passed and retested**; lint, TypeScript and production build passed. Existing lint warnings remain; no frontend product source changed. |
| Rollout and harness | **70 behavioral rollout/probe tests** passed and retested; **4** bounded CheckTx retry/reconciliation tests passed and retested. |
| Static/artifacts | Contract Clippy, formatting, Python compilation, shell syntax, documentation crosslinks and whitespace checks passed. Optimized wasm built with optimizer 0.16.1 / Rust 1.81.0, stripped linking and `wasm-opt -Os --signext-lowering`. |

The cache API regression now requires exactly one live ContractInfo GET per
candidate on a cache hit, with unchanged quote/hops and no renewed simulation,
capability or pool queries. This replaces its previous all-LCD-count equality,
which conflicted with immediate code-identity invalidation.

Real-chain tests caught strict response projections rejecting unrelated factory
and router config fields. Response projections now accept those fields while
retaining all required security-field checks. Fixtures include the real response
shape. Candidate custody additionally prevents a reviewed-code copy controlled
by an attacker from gaining authority and later migrating itself.

## Actual SDK / LocalTerra receipts

Only fresh synthetic wallets/contracts and LocalTerra funds were used. Shared
chain/application contracts and volumes were untouched. The isolated chain used
a persistent RAM-backed test directory after shared infrastructure stalled.
Fixtures create dedicated owner/sink keys; public evidence contains test-key names
and addresses, never private keys or mnemonics. Retries are bounded and occur only
after rejected CheckTx sequence errors; accepted/unknown outcomes are reconciled.

- [Pre-funded 1.1 → final 1.3 migration](evidence/pre-v1-funding-final.json):
  **6 funding/rollback checks**, **60 receipts**; original addresses, balances,
  configuration and pool reserves preserved; live pair pins refreshed.
- [Funded 1.2 → final 1.3 migration](evidence/v12-funding-final.json):
  **6 funding/rollback checks**, **60 receipts**, including zero intermediate
  wallets, multiple taxed hops, existing router dust and minimum-return rollback.
- [Invoice/native flows](evidence/pre-v1-flows-final.json): **5 checks**, **18
  receipts**, including atomic invoice failure/success, native wrap failure/success
  and taxed-intermediate native unwrap.
- [Recovery after pre-funded migration](evidence/pre-v1-trust-final.json) and
  [recovery after funded migration](evidence/v12-trust-final.json): **12 checks
  and 23 receipts each**. Token admin is changed to an opaque contract; direct
  factory-governance rotation works without a dispatcher. Unauthorized caller,
  wrong hash/factory and attacker-owned reviewed code fail atomically; stale
  routing fails; revoke and subsequent recovery succeed.

Final actual stored artifact hashes match the reviewed
[checksum manifest](../../../smartcontracts/artifacts/checksums.txt):

| Artifact | SHA-256 |
|---|---|
| Router 1.3.0 | `9cc854e42d786eee5040d887b3c06eaebd23c27af4637bb19cadaf9cea43eddd` |
| Community-tax-token 1.3.0 | `55ba5b89e6c7f5269caca2751a640db9b3e57adb51e769e443e5f28107d5d33f` |

The rebuilt launcher hash also changes through its token dependency. Retain the
compatible deployed 1.0.0 launcher and rotate its template; do not force a
same-version/different-hash launcher migration. A future launcher wasm replacement
requires a properly versioned reviewed release.

## Acceptance checklist

- [x] **AC1:** complete inventory preflight rejects new/stale router bindings;
  real existing-address migrations preserve funded routing.
- [x] **AC2:** adopted-instance unit recovery and real-chain opaque-admin recovery,
  candidate identity/custody validation, stale-call rejection, atomic failure and
  overlapping-role preservation.
- [x] **AC3:** same-version/different-hash artifact rejection, actual recorded
  code hashes, and immediate discovery cutover on live code changes.
- [x] **AC4:** canonical single emission, duplicate-slot/stale-route rejection,
  identical transfers in separate same-height routes, bounded storage, router
  dust isolation and whole-transaction rollback.
- [ ] **Production rollout/recovery:** no production broadcast, migration,
  rotation or bounded live swap was performed. Synthetic receipts establish test
  acceptance only. Production execution remains unverified under #1384.

## Reproduce and evidence scope

Follow [README.md](README.md) for infrastructure, exact wrapper commands and
skip variables. The executed local chain commands were:

```bash
python3 scripts/qa/localterra-route-tax-1381.py --container cl8y-dex-issue1388-memory \
  --legacy-artifacts /tmp/dex-1388/pre-v1-artifacts \
  --code-cache /tmp/dex-1388/final-manual-codes.json \
  --evidence-file /tmp/dex-1388/pre-v1-funding-final.json
python3 scripts/qa/localterra-route-tax-flows-1381.py --container cl8y-dex-issue1388-memory \
  --fixture /tmp/dex-1388/pre-v1-funding-final.json --stack-env /tmp/dex-1388/wrap.env \
  --evidence-file /tmp/dex-1388/pre-v1-flows-final.json
python3 scripts/qa/localterra-router-trust-1388.py --container cl8y-dex-issue1388-memory \
  --fixture /tmp/dex-1388/pre-v1-funding-final.json \
  --evidence-file /tmp/dex-1388/pre-v1-trust-final.json
```

Repeat using the reviewed 1.2 artifacts and fresh funding fleet for the funded
migration/recovery path. Supply local reviewed artifact paths and an existing
public LocalTerra wrap stack when reproducing. Recorded receipts are validated by
`python3 scripts/qa/check-issue-1388-evidence.py`; that command checks recorded
identity and outcomes, and does not execute fresh transactions or prove production.
Agents may skip unavailable checks and report **PASS with explicit skips**, with
concrete reasons and unchecked criteria. Executed assertion failures remain FAIL.
