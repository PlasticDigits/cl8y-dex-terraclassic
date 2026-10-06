# Issue #1394 — accepted implementation receipt

Issue: [#1394](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1394).
[Review/agent checklist](README.md), [PD1394-1–PD1394-10](../../../skills/AGENTS_PAIR_DISCOVERY_CACHE.md).

Tested implementation and fixture revision: `d5ee494c9f0db4ec295d6c8b4ebb06e2e993f160`.
The following documentation-only receipt commit leaves that tested source unchanged.
UTC: **2026-10-05T07:24:36Z → 2026-10-05T07:25:07Z**. Result: **PASS — 23 checks, 0 failures, 0 skips**.

Executed from the isolated `cl8y-dex-terraclassic-1394` worktree:

```bash
CARGO_HOME=/dev/shm/cl8y1394-cargo \
CARGO_TARGET_DIR=/dev/shm/cl8y1394-build \
CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=8 make verify-issue-1394
```

The verifier prints exact expanded stage commands; its standard Cargo commands
and the agent skip policy are in [README.md](README.md). A dedicated Postgres 16
fixture database, isolated test lock and temporary memory-backed data directory
were used. Cargo ran on the host. These tests check database contents and
provenance, not crash durability; no production chain or deployment was changed.

| Check | Actual result |
|---|---|
| Changed Rust formatting | PASS |
| Discovery/cache/admission regressions | 29 passed; manual benchmark excluded from this stage |
| Focused default parallel | Eight runs × seven tests; zero failures |
| Focused 32-thread parallel | Eight runs × seven tests; zero failures |
| Full default-parallel library | 491 passed; one manual benchmark ignored here and executed separately |
| Hot lookup / capacity churn benchmark | PASS; one benchmark test |
| Discovery/cache/admission retest at 32 threads | 29 passed |
| Postgres discovery | Nine passed |
| Postgres discovery retest | Nine passed |

Synthetic debug-profile benchmark output (no environment-dependent timing threshold):

```text
pair-discovery cache: entries=1 targeted_lookups=2000000 elapsed=553.652474ms
pair-discovery cache: entries=512 targeted_lookups=2000000 elapsed=548.554341ms
pair-discovery cache: capacity_churn=100000 retained=512 elapsed=121.317478ms
```

The 100,000-admission state regression also asserts the full ceiling of 16
factory namespaces, 512 entries and exactly one FIFO record per resident key
per factory. Production admission uses a burst of 32, refill of eight per second
and four in flight per factory; injected-clock tests verify refill and fairness.

Completed issue criteria:

- [x] AC1: one LCD/factory rejection cannot suppress a different namespace's legitimate pair; both rejection and successful Postgres persistence tested.
- [x] AC2: parallel discovery tests are deterministic without process-global clears; all repeated default/high-thread runs passed.
- [x] AC3: memory/work remains bounded under churn, and cached foreign pairs avoid repeated pair/factory queries.
- [x] AC4: timeouts/failover do not create negative entries; direct errors, cached reason, admission outcomes and fixed counter snapshots stay observable.
- [x] Invalid Bech32/HRP/variant/payload, oversized response, concurrency/cancellation and admission-wait regressions.
- [x] Invariants, agent checklist, skip policy, indexer/testing/AGENTS crosslinks and `make verify-issue-1394` documented.

Additional checks: host indexer `cargo clippy --locked --offline --lib` exited 0
(existing warnings in unrelated modules remain; none referenced the changed
pair-discovery modules). Commit hooks ran normally: contract formatting,
contract Clippy with `-D warnings`, commit-message validation and staged
Gitleaks all passed. The hook's frontend stage skipped because this isolated
worktree has no frontend dependencies; no frontend source changed.

## Failed attempts and correction

An earlier post-commit receipt on `b1e7dd8773cc9af0136b6eec8f3dcbc842b52f08`
ended **FAIL (22 checks passed, one failed)** at 2026-10-05T07:16:11Z:
a Postgres timeout fixture allowed only 80 ms for its healthy recovery request.
Eight of nine tests passed in that stage; the healthy request also timed out.
That failed receipt was not accepted as PASS. The fixture now separates a
one-second client timeout from a two-second forced response delay; unit timeout
fixtures and elapsed-work assertions likewise tolerate scheduling variation.
The complete verifier passed after correction, then passed again on the exact
revision recorded above.

Earlier Postgres stages were interrupted while waiting on a shared fixture lock
and disk-backed migration sync; their interrupted runs were not accepted as
PASS. A dedicated lock and memory-backed database resolved the infrastructure
blockage. Initial commit-hook cache misses were corrected by attaching the
existing dependency caches, and hooks were rerun without bypassing them.

## Limits and unverified scope

No issue acceptance criterion was skipped. Production deployment, a live discovery
flood, remote chain attestation, wallet/device QA and crash durability were not
performed and are not claimed. Namespace identity is client-local; configured
failover endpoints must serve the same chain. The getter retains its existing
warning/soft-skip policy for transient discovery failures: later events retry
without poisoned evidence, but this fix does not add replay of an already
soft-skipped event. Temporary admission pressure waits instead of introducing
additional successful skips.

SHA-256 of the nine implementation/test/verifier files listed below, in order,
using `path + NUL + file bytes` for each: `5ac85ed486c891fa8ea5df49bbf693fabb17380dc473fa5746603a4e7100b24f`.

```text
indexer/Cargo.toml
indexer/Cargo.lock
indexer/src/config.rs
indexer/src/indexer/pair_discovery.rs
indexer/src/indexer/poller.rs
indexer/src/lcd/mod.rs
indexer/src/lcd/pair_discovery_state.rs
indexer/tests/indexer_pair_discovery.rs
scripts/verify-issue-1394.sh
```
