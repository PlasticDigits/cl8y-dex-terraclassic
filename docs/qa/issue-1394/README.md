# Issue #1394 — pair discovery implementation review

Issue and comments: [#1394](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1394).
Invariants and third-party agent instructions:
[PD1394-1–PD1394-10](../../../skills/AGENTS_PAIR_DISCOVERY_CACHE.md).
Implementation: [pair discovery](../../../indexer/src/indexer/pair_discovery.rs),
[LCD client](../../../indexer/src/lcd/mod.rs),
[state and admission](../../../indexer/src/lcd/pair_discovery_state.rs).
Database regression:
[indexer_pair_discovery.rs](../../../indexer/tests/indexer_pair_discovery.rs).
Production admission waiting and aggregate telemetry:
[poller](../../../indexer/src/indexer/poller.rs) and
[parser](../../../indexer/src/indexer/parser.rs).

The October 4 investigation reproduced two failures in eight default-parallel
focused runs, while the serial baseline passed 470 library tests. The October 5
issue comment explicitly says that preserved audit reports and serial sanity
tests do not implement remediation. This fix must remove shared address-only
rejection state and bound discovery work while preserving factory provenance.
The reproduction is a test reliability finding and conditional availability
finding; it does not demonstrate theft or a public unauthenticated HTTP trigger.

## Run and retest

Use an isolated git worktree and Rust on the host. PostgreSQL is needed only for
the integration stage. Provision a dedicated test DB with
`make setup-indexer-postgres` or the
[Postgres playbook](../../../skills/AGENTS_LOCAL_POSTGRES_DEV.md). Run integration
tests from `indexer/` so its `.env` is loaded. Never run Cargo in a root Docker
container bind-mounted onto this repository.

```bash
make verify-issue-1394
```

The [verifier](../../../scripts/verify-issue-1394.sh) runs Cargo with `--locked`
to preserve the reviewed dependency lockfile. It checks changed Rust source
formatting, cache/admission regressions, eight focused runs at default parallelism
and eight at 32 threads, the full default-parallel library, hot-lookup/churn
benchmark, targeted retest and two serial Postgres discovery runs. Serializing
the DB fixture prevents collisions in its dedicated test database; it does not
replace parallel library verification. To reproduce stages directly:

```bash
cd indexer
cargo test --locked --lib pair_discovery
# Repeat each focused command eight times (default verifier settings).
cargo test --locked --lib discover_new_pair
cargo test --locked --lib discover_new_pair -- --test-threads=32
cargo test --locked --lib
cargo test --locked --lib pair_discovery_cache_benchmark -- --ignored --nocapture
cargo test --locked --lib pair_discovery -- --test-threads=32
cargo test --locked --test indexer_pair_discovery -- --test-threads=1
cargo test --locked --test indexer_pair_discovery -- --test-threads=1
```

`VERIFY_ISSUE_1394_REPEATS` and `VERIFY_ISSUE_1394_THREADS` accept positive integers.
Keep at least eight repetitions and 32 high-parallel threads for issue acceptance;
a smaller smoke run is not the repeated-parallel receipt. The benchmark reports
synthetic elapsed times and asserts bounded state and query/admission behavior;
timing results are environment-specific, not a production throughput guarantee.

The production getter retries temporary factory and shared-provider admission
deferrals at 125 ms intervals, including typed `LcdError::AdmissionDeferred`
from rate/concurrency/half-open capacity pressure (#1453),
rechecking the DB before each attempt. Provider capacity waits retain completed
identity, pair, provenance and metadata phases; direct discovery remains immediate.
Run `getter_preserves_discovery_phases_under_two_token_provider_burst` to prove a
small shared budget makes progress without repeating successful smart queries. A concurrent successful discovery can
therefore satisfy waiting callers without another LCD query. Cached DB pairs
bypass admission. Invalid factory configuration and permanent admission deferrals
remain distinct errors. There is no internal event queue or claim of unlimited
flood throughput: admission pressure can slow block progress, while keeping
discovery state bounded and avoiding newly introduced admission-related event
drops. Fixed-field discovery counters are publicly readable on the LCD client and
logged in aggregate by the production poller every 100 processed blocks.

[Provider recovery #1464](../../../skills/AGENTS_PROVIDER_RECOVERY.md) supersedes
legacy getter soft-skips for typed provider availability/integrity and database
persistence failures: they propagate to pending-block retry or integrity halt, so
failed economic events cannot be successfully omitted. Ambiguous foreign/non-pair
contract-local malformed responses retain their legacy `None` policy. Neither path
creates negative evidence. Direct discovery retains typed errors/counters. Local
tests do not establish production flood acceptance or completeness during outages.

## Reviewing agent checklist

For each row, run the indicated tests, inspect the named invariant in source and
record actual outcomes. Check an item only after its evidence succeeds. The full
verifier output and benchmark output should accompany acceptance receipts.

| Completed | Criterion | Agent action and evidence |
|---|---|---|
| [ ] | AC1: namespace isolation | Test the same valid address across independent LCD clients and different factories: one rejects, another accepts. Verify clones share state; rebuilding a client for an endpoint/chain change isolates it. Review PD1394-1/2 and exact factory-listed address equality. Do not describe the local namespace as chain attestation. |
| [ ] | AC2: deterministic parallel tests | Run `discover_new_pair` eight times each at default and 32-thread parallelism, and full `cargo test --lib`. Require zero failures. Confirm tests have no process-global clear and no global shared rejection map. |
| [ ] | AC3: bounded cache/work and repeat suppression | Run cache capacity/TTL/FIFO tests and ignored benchmark. Confirm 16 factories × 512 rejects = 8,192 maximum, bounded queues, one-hour expiration, lookup without full-map scans and constant bounded insertion work. Assert a known foreign pair avoids repeated pair/factory LCD calls. |
| [ ] | AC4: transient outcomes observable | Run timeout/failover, deterministic schema rejection and factory mismatch tests, including `getter_provider_failure_propagates_and_direct_discovery_preserves_error` and `factory_transient_error_type_is_preserved_and_getter_retry_succeeds`. Assert direct discovery retains typed transient errors/counters, the getter propagates typed infrastructure errors while keeping ambiguous contract-local skips, and retry performs provenance again after failure; only `ContractQueryRejected` permits deterministic text matching. Check stored/cached `Unlisted` versus `NotAPair` and typed admission deferral distinct from rejection. |
| [ ] | Supported persistence/recovery | Run the Postgres discovery test twice with a dedicated DB. Assert a factory-listed pair persists, foreign pairs do not, a rejected namespace cannot block legitimate persistence and transient retry can succeed. Run `getter_retries_provider_admission_pressure_without_dropping_pair` and `getter_waits_for_coalesced_discovery_without_dropping_legitimate_pair` to check production admission waiting and DB recheck; exact rate/refill behavior uses the injected-clock state tests. Confirm transient errors remain visible through direct discovery/counters, typed provider/persistence failures propagate, ambiguous contract-local skips stay uncached, and pending-block recovery retries without an event omission. Run `recovery1464_discovery_provider_integrity_and_database_failures_are_not_event_skips`. |
| [ ] | Attack: invalid addresses | Test bad checksum, case, size, empty factory and valid-checksum wrong-HRP, Bech32m and unsupported decoded payload lengths. Assert invalid requests fail before LCD calls; valid 20/32-byte Terra contract addresses still work. An invalid event pair may be skipped, while invalid factory configuration must return an error. |
| [ ] | Attack: foreign-event flood/fairness | Run repeated rogue-address, rotating-address, concurrent admission/cancellation and clock-advance tests. Assert a per-factory burst of 32, refill of eight attempts/second and four in flight, same-pair coalescing, independent retained factory budgets, permit release and deferred retry. Assert admission waits make no LCD calls and already-persisted legitimate pairs bypass admission. The seventeenth factory is deferred; slots are never evicted to reset budgets. Admission pressure may slow progress but must not itself introduce successful skips of legitimate events. Record the separate ambiguous contract-local skip scope under PR1464. |
| [ ] | Attack: response size and telemetry | Run `discover_new_pair_oversized_response_remains_retryable`, LCD bounded-body regressions and counter tests. Review the 64 KiB cap, oversized response retryability, initial/cache/deferral fixed outcome counters, public snapshot and poller aggregate log every 100 blocks. Confirm no per-address metric maps or per-wait warning flood. Never run an unapproved production flood. |

## Skip policy and evidence scope

Agents may skip checks they cannot complete and still mark overall verification
**PASS with explicit skips**. Complete all runnable checks first and attempt
feasible provisioning; record each unavailable check, concrete reason, attempted
command or infrastructure evidence, revision and UTC time. Leave its checklist
entry unchecked/unverified. No unavailable workflow hold or verification gate
blocks the implementation under this policy. An executed assertion, compilation
or formatting failure remains **FAIL**; fix and retest it or report FAIL.

If PostgreSQL remains unavailable after a concrete setup/probe attempt, run:

```bash
VERIFY_ISSUE_1394_SKIP_POSTGRES_REASON='Concrete setup/probe failure and evidence' \
  make verify-issue-1394
```

The reason variable skips only the unavailable DB stage; the runnable library,
benchmark and format stages still execute. For other unavailable tools, apply
the same documented policy manually and enumerate which stages were unexecuted.
Passing checks establish implementation scope only. No deployment, wallet/device
action or production discovery flood is required for this issue's source fix;
these are not inferred from local unit, mock-LCD or Postgres tests.

## Receipt template

Keep published receipts synthetic and secret-free. Do not copy `.env`, tokens,
endpoint credentials or wallet mnemonics into reports. Record the final revision
after committing; verifier receipts from a dirty worktree must be linked to that
revision and the reviewed patch.

```text
Revision: <git rev-parse HEAD>
UTC start/end: <date -u +%Y-%m-%dT%H:%M:%SZ>
Exact command: <environment overrides without secrets; command and working directory>
Outcome: <PASS / FAIL / PASS with explicit skips; executed test counts>
Parallel receipts: <8 default + 8 high-thread runs, zero failures or actual failures>
Benchmark: <hot lookup / capacity churn output; bounded-state assertions>
Postgres: <dedicated DB fixture results; twice or explicit unavailable reason>
Skipped/unverified: <each criterion, attempted command, reason and evidence>
Evidence scope: synthetic tests; no production deployment/flood acceptance
```

Final implementation receipts belong in [RESULTS.md](RESULTS.md). General test
entry points: [testing](../../testing.md) and
[indexer invariants](../../indexer-invariants.md).
