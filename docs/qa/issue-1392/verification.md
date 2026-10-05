# Verification record — issue #1392

Issue [#1392](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1392).
Base revision: `9944ada10ee711ff8ffaa91f4c19b0f292de3924`.
Branch: `fix/issue-1392-chain-attestation`. Verified 2026-10-05 UTC.
**Overall source verification PASS: 12 groups passed, zero failed, zero skipped.**
[Machine results and code-scope digest](results.json), [agent checklist](README.md),
[NET1392 invariants](../../../skills/AGENTS_CHAIN_ATTESTATION.md) and
[endpoint policy](../../runbooks/chain-attestation.md).

## Executed checks and retest

| Check | First | Retest |
|---|---|---|
| Entire indexer unit suite | 482 PASS | 482 PASS |
| `cargo check --tests` | PASS | PASS |
| Focused PostgreSQL API/ingestion tests | 39 PASS | 39 PASS |
| Treasury chain/credential/retry tests | 13 PASS | 13 PASS |
| Existing cUSTC buyback failure/retry tests | 7 PASS | 7 PASS |
| UST1 buyback, oracle rebalance, cLUNC/cUSTC and cLUNC/USDT math/shell regressions | PASS | PASS |
| Documentation links/invariants/env generation | PASS | PASS |

Shell syntax, changed-source whitespace and new identity-module formatting passed.
Independent source review and adversarial HTTP/transaction mocks passed, including
credential-path receipts and sparse/duplicate fallback selection. Existing Rust
unused/dead-code warnings remain outside this remediation; no executed final
assertion failed.

API binaries: `api_health` (9), `api_fee_discount_health` (5),
`api_limit_book_lcd_mock` (2), `api_route_tax_funding1381` (6),
`book_snapshot_loop` (6), `indexer_ingestion_hardening` (11).
The database and process lock were dedicated to this issue. Mutable Cargo metadata
and workspace artifacts were made private before the final builds.

Reproduce with an isolated test database configured in `indexer/.env`:

```bash
CARGO_HOME=/tmp/cargo-1392 CARGO_BUILD_JOBS=2 \
INDEXER_CARGO_TARGET_DIR="$PWD/indexer/target" \
VERIFY_ISSUE_1392_LOCAL_COMMAND='python3 scripts/lib/chain-attestation.py --expected-chain localterra --lcd http://172.17.0.13:1317 --rpc http://172.17.0.13:26657' \
VERIFY_ISSUE_1392_LIVE_COMMAND='python3 scripts/lib/chain-attestation.py --expected-chain columbus-5 --lcd https://terra-classic-lcd.publicnode.com --rpc https://terra-classic-rpc.publicnode.com:443 --contracts terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea' \
make verify-issue-1392
```

Choose your own private Cargo cache and local endpoint addresses. `/tmp/cargo-1392`
was an isolated local registry cache; it is not a repository dependency. The local
chain used the pinned repository LocalTerra image and an isolated temporary home,
without changing any shared chain or preparing a transaction.

## Issue criteria completed

- [x] **AC1:** Wrong/missing/malformed/unavailable identity cannot reach contract
  handlers or populate discovery/quote caches. All treasury preparation stages
  have zero calls after failed preflight. Tests also reject oversized identities.
- [x] **AC2:** Each selected/fallback/recovered endpoint rechecks identity;
  redirects and contradictory block headers cannot bypass it. Health snapshots
  qualify expected/observed identity, verification, time and private cache scope.
  Identical addresses/code numbers on different expected chains stay separate.
- [x] **AC3:** Economic buyback/LP/treasury scripts use the shared policy, including
  each tick, dynamic route identities, direct generation/sign/multisign/broadcast,
  retry, endpoint override and dry-run branches. Code/hash/height/block mismatch,
  stale/future headers and mid-run code/network rotation fail closed.
- [x] **AC4:** Correct Columbus-5 and explicit LocalTerra reads/dry-run mocks pass.
  Actual read-only probes pass; the fresh isolated local chain uses the unchanged
  default 120-second policy. Its shared-host dry-run returns `DRY_RUN_TX` without
  calling signing/broadcast. Existing spend/quote/custody controls remain intact.
- [x] **Abuse/residuals:** Slow code reads cannot outlive the anchor; URL credentials
  stay out of receipts/headers. Initial agreeing malicious providers remain a
  documented residual, not cryptographic RPC authentication.
- [ ] **Deployed revision and real signing/broadcast:** UNVERIFIED. This task
  publishes source/PR; no production release, wallet ceremony or treasury tick
  was performed. Read-only/unit PASS does not check this box.

## Read-only evidence

Final source: [Columbus-5 factory/block](columbus5-final-receipt.json),
[fresh isolated LocalTerra block](fresh-localterra-final-receipt.json).
Both use default freshness and exclude endpoint URLs from result payloads.
Earlier [six-core-contract corroboration](columbus5-protocol-receipt.json) checks
factory/router/UST1/cUSTC/vFDUSD/treasury over publicnode LCD and Hexxagon RPC;
[factory](columbus5-factory-receipt.json) and [network](live-receipt.json) probes
provide additional public observations. They are working-tree receipts, never
proof of installed production code.

Earlier shared-local [factory](localterra-factory-receipt.json) and
[bounded-age](localterra-final-receipt.json) observations are historical. That
shared clock aged behind the host and its container subsequently stopped.
An isolated fresh chain replaced it for final verification, removing the need
for any age override ([initial fresh probe](fresh-localterra-receipt.json)).

Earlier executed failures were retained and fixed, never converted to skips:
mock-builder priority order, restoring identity fixtures after mock reset, an
old exact health-body assertion, and the expected 503 after discovery cooled
an unavailable endpoint. The buyback mock subprocess timeout was increased to
240 seconds and the full suite passed twice. Interrupted builds and unsafe
shared cache seeding were discarded before the final private build.

## Agent skip and release instructions

Follow [the checklist](README.md): complete runnable checks first, record commands,
UTC/revision/digest and scope, then fix/retest every failure. An unavailable check
may be **SKIP** with a specific attempted prerequisite and reason; leave its
criterion unchecked. Agents may mark overall PASS with explicit skips when all
completed checks pass. A failed assertion remains FAIL until corrected and
retested; a skip never certifies deployment or historical chain authenticity.

For release, set `EXPECTED_CHAIN_ID=columbus-5` on the production indexer before
rebuilding/restarting; local deploy writes `localterra`. No schema/contract
migration is required by this fix. Review optional immutable pins when protocol
code changes; intentional treasury migrations retain their documented exception.
Production acceptance and current financial state require separate receipts.
