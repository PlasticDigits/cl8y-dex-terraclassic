# Issue #1399: agent review and verification

[#1399](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1399) implements
unfillable active-row rejection and bounded legacy eviction left unresolved by
closed [#1225](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1225).
Invariants **UF1399-1–UF1399-10** are defined in the
[agent playbook](../../../skills/AGENTS_UNFILLABLE_LIMIT_ORDERS.md), with behavior in
[limit-orders.md](../../limit-orders.md#unfillable-in-band-floor-orders-forgejo-1225)
and security invariant [L26](../../contracts-security-audit.md).

## Automated checks

```bash
make verify-issue-1399
```

The script runs focused common/pair/indexer tests twice, the full contract
workspace once, DB hybrid route integration once, formatting and documentation
checks. It exits nonzero on any executed failure. Shared build caches can be
selected with `CONTRACTS_CARGO_TARGET_DIR` and `INDEXER_CARGO_TARGET_DIR`;
optional `CONTRACTS_CARGO_HOME` and `INDEXER_CARGO_HOME` preserve each build
cache's dependency paths when concurrent agents need separate Cargo homes.
Provision Postgres with `make setup-indexer-postgres` if DB integration is needed.
Provision LocalTerra with `make setup-cloud-localterra` before declaring a local
chain unavailable. Unit and DB fixtures use synthetic identities only.

Unavailable groups accept an explicit reason via
`VERIFY_ISSUE_1399_SKIP_CONTRACTS_REASON`,
`VERIFY_ISSUE_1399_SKIP_FULL_CONTRACTS_REASON`,
`VERIFY_ISSUE_1399_SKIP_INDEXER_REASON`,
`VERIFY_ISSUE_1399_SKIP_DB_REASON`,
`VERIFY_ISSUE_1399_SKIP_FORMAT_REASON`, or
`VERIFY_ISSUE_1399_SKIP_DOCS_REASON` (for example, an unavailable Python runtime).
Use these only after establishing the group cannot run in the current environment.
For example, unavailable Postgres after provisioning attempts may be recorded with
`VERIFY_ISSUE_1399_SKIP_DB_REASON='Postgres unavailable: <observed reason>'`.
This is permission for agent skips, not permission to conceal executed failures.

## Review checklist and issue criteria

The reviewing agent records PASS, FAIL or SKIP with evidence for every row, then
checks off only rows actually verified. Implementation claims alone do not check these boxes; record the test receipt
that supports each checked criterion.

- [x] **AC1:** Single/batch/ladder placement and repricing reject unfillable rows;
  inspect post-fee atomicity and ask/bid 6/6, 6/18, 18/6 payable boundaries plus
  checked overflow. A valid row must not be evicted for a small taker budget.
- [x] **AC2:** Under zero clean thresholds, legacy unexpired invalid rows unlink
  into `reason=unfillable` claims; valid orders retain FIFO. Inspect pending
  escrow before/after park, owner claim, cancellation and double-refund rejection.
- [x] **AC3:** Seed more than `MAX_SCAN_STEPS` invalid heads followed by useful
  liquidity. Repeated bounded matching/clean pagination must make persistent
  progress with scan, park and maker caps unchanged. Include expired/dust rows
  sharing the park budget, and a failed transaction that rolls back cleanup.
- [x] **AC4:** Execute, contract simulation and indexer agree for matching fixed
  snapshots, rounding and cap exhaustion. Existing no-zero-payout, expired-prefix,
  FIFO, blacklist/pause/freeze and hybrid tests stay green.
- [x] Run the verifier, correct failures, and run its focused retest; save revision,
  UTC timestamp, exact commands, outcomes, test counts and evidence scope.
- [ ] If available, build optimized wasm, deploy/migrate on isolated LocalTerra,
  and reproduce synthetic clean/claim lifecycle and gas measurements. Record
  this as SKIP with reason when unavailable; the source harness is not chain evidence.
- [ ] Deployment acceptance requires separately recorded deployed code IDs,
  migration receipts and version-specific checks. Public-chain attack fixtures
  are unnecessary. Record SKIP if custody/deployment is unavailable.

## Explicit skip policy

Agents may skip infrastructure, signing, deployment, device or other checks they
cannot complete and mark overall verification **PASS WITH SKIPS**, provided all
executed checks pass. List each skipped item and its observed reason. A failed
executed check remains **FAIL** until fixed and retested; a later unavailable
rerun does not erase it. A source PASS WITH SKIPS does not mean omitted criteria
were verified, production is deployed, or this is third-party audit certification.
Complete available checks first and do not leave a verification-only hold on an
otherwise complete implementation. Production deployment is outside this implementation PR scope and is not performed
by this source verifier.

## Evidence receipt

Verified on 2026-10-05 at 08:48 UTC against base revision
`9944ada10ee711ff8ffaa91f4c19b0f292de3924` plus this PR's implementation.
The source/test diff SHA-256 is
`4f98fb080c92538a622893adbf6115d6813f6e05c97cb35653fdfec42ff5345e`.
Reproduce the digest with `git diff --binary 9944ada10ee711ff8ffaa91f4c19b0f292de3924 HEAD -- smartcontracts indexer | sha256sum`.

Exact verification command from the worktree root (cache overrides are optional):

```bash
CONTRACTS_CARGO_HOME=/dev/shm/cargo-issue1399-contracts \
INDEXER_CARGO_HOME="$HOME/.cargo" CARGO_NET_OFFLINE=true \
CARGO_BUILD_JOBS=2 TEST_DB_LOCK_FILE=/tmp/cl8y-dex-1399-db.lock \
make verify-issue-1399
```

| Check | Outcome |
| --- | --- |
| Focused contract checks, first run and retest | PASS each: 17 placement, 4 park-reason, 5 pair economics/zero-claim, 5 public placement/claim, 2 no-zero-cost, 1 expired-prefix tests |
| Indexer DB book simulation, first run and retest | PASS: 27 tests each, including both-side mixed-decimal boundaries and rounded reciprocal parity |
| Full contract workspace | PASS: 897 tests, including all 550 integration tests and 97 pair tests |
| DB hybrid route integration | PASS: 9 tests, including mixed legacy prefixes and the exact in-band minimum ask price |
| Changed Rust formatting, documentation and invariant crosslinks | PASS |
| Automated verifier total | **8 PASS, 0 FAIL, 0 SKIP; overall PASS (source scope)** |
| Strict contract Clippy | PASS: common/pair, all targets, `-D warnings` |
| Skip/failure accounting probes | PASS: unavailable groups return PASS WITH SKIPS; injected executed failures return FAIL/nonzero |

Strict lint command:

```bash
CARGO_HOME=/dev/shm/cargo-issue1399-contracts \
CARGO_TARGET_DIR="$PWD/smartcontracts/target" CARGO_BUILD_JOBS=2 \
cargo clippy --offline --manifest-path smartcontracts/Cargo.toml \
  -p dex-common -p cl8y-dex-pair --all-targets -- -D warnings
```

The initial full workspace run found four new fixture/assertion failures. A
representable reciprocal-zero price fixture and a dedicated
`LimitOrderUnfillable` diagnostic corrected them; all affected checks and the full
workspace passed on retest. No executed failure was skipped. An interrupted
verifier was rerun to completion rather than counted as a pass.

DB checks used a dedicated synthetic database. After observing disk-sync stalls,
a separate PostgreSQL 16 container with memory-backed test storage completed the
same migrations and test suite; durability was disabled only for that disposable
fixture instance. Shared LocalTerra and production settings were unchanged.
Indexer builds still emit pre-existing unused/dead-code warnings outside this fix;
strict common/pair lint and changed-file formatting passed.

**Unverified optional rollout scope:** optimized-wasm/isolated-chain migration,
gas measurements, wallet/device signing and production deployment were not
performed by this implementation PR. The shared local chain was probed but not
migrated. These unchecked rollout items may be recorded as SKIP for overall agent
review **PASS WITH SKIPS**; they are not deployment evidence. After merge, deploy
the updated indexer and migrate pair wasm through the existing migration workflow,
then verify version-specific clean/claim behavior and gas before live acceptance.
