# Integrated release operations (#1421)

Use this when checking the 2026-10-06 integrated Columbus-5 release, its
indexer/frontend pins, database role split, or the remaining custody receipts.
Issue: [#1421](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1421).
Checklist: [docs/qa/issue-1421/README.md](../docs/qa/issue-1421/README.md).

Route migration evidence is [#1384](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1384)
and [RTROLL1381-1–RTROLL1381-10](AGENTS_ROUTE_TAX_ROLLOUT.md).
Database boundaries are [DB1396-1–DB1396-10](AGENTS_INDEXER_DATABASE_ISOLATION.md).
Bridge-Debt market pins are [BD1393-1–BD1393-10](AGENTS_BRIDGE_DEBT_MARKET_PINS.md).
The pair fleet issue remains [#1386](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1386).

| ID | Required behavior |
|---|---|
| REL1421-1 | The integrated contract release is factory 1.12.0, pair 1.21.0, router 1.5.0, community-tax token 1.4.0, and AutoLP 1.1.0. Standalone branch artifacts are not this release. |
| REL1421-2 | Serving indexer and frontend stay on the pinned application revision whose runtime inputs match `application-source.json`. Later operational commits are allowed only when those inputs and the contract sources are unchanged. |
| REL1421-3 | Production expects `columbus-5` and fails over only among same-chain endpoints. One endpoint returning 429 does not erase another endpoint that verified `columbus-5`. |
| REL1421-4 | Runtime and migrator logins differ. Production acceptance is the maintenance receipt: preserved migration history, schema marker, and actual permission denials. `make verify-issue-1396` is the local suite and does not replace that receipt. |
| REL1421-5 | `bash scripts/ops/2026-10-06/route-03-verify.sh` may return contract PASS with `production_execution` UNVERIFIED. A funded swap receipt is a different check. |
| REL1421-6 | Bridge-Debt 1.1.0 migration, finite paused limits, one-raw-unit repayment, and original-payer recovery require the encrypted keyring. A `planned` journal plus live 1.0.0 bindings is not that migration. |
| REL1421-7 | Measured eight-hook gas on the dedicated LocalTerra fixture is not an arbitrary-hook ceiling and not a production provider query envelope. |
| REL1421-8 | Schema-2 evidence keeps public transaction linkage and omits actor identifiers. That is not anonymity. |
| REL1421-9 | Identical native release bytes are not whole-image reproducibility. |
| REL1421-10 | Finish every read-only check first. A keyring or device check may be SKIP when it cannot be run; the skip stays written. An executed failure stays FAIL. Skipping custody does not prove the skipped receipt. |

Read-only commands from a checkout that has the reviewed wasm artifacts:

```bash
bash scripts/ops/2026-10-06/route-03-verify.sh
python3 scripts/ops/2026-10-06/bridge-debt.py plan
python3 scripts/ops/2026-10-06/route-provider-readback.py
```

The optional keyring receipt, which reuses completed DEX transactions and then
runs the funded smoke and Bridge-Debt stages:

```bash
bash scripts/ops/2026-10-06/run-custody.sh
```

`make verify-issue-1421` checks these cross-links. It does not query Columbus-5
or production Postgres.
