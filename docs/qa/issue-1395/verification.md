# Verification receipt: issue #1395

Result: **PASS for implementation and isolated synthetic acceptance**.
Recorded 2026-10-05 08:16:12 UTC. Branch: `fix/issue-1395-strict-event-amounts`.
Base revision: `9944ada10ee711ff8ffaa91f4c19b0f292de3924`. The commit containing this receipt identifies the tested
implementation; PR head links the published revision.
Staged functional patch SHA-256 (`git diff --cached --binary -- indexer scripts Makefile`):
`1d8bfb9b004fa1475d0a25fd26b9c57152b45679710977d3681e6153c8ab4eb3`.

Issue/comments were read with the openbao-tools repository's `workstation/bao-exec`
using its selected Forgejo credential and `fj issue view 1395` / `comments`.
The October 5 comment confirmed main contained findings, not remediation.
Three delegated agents implemented parser/fee/test work and independently reviewed
scope, provenance, replay keys and numeric bounds. Review findings were corrected
and covered by regressions before final verification.

## Commands and results

The worktree used a dedicated PostgreSQL 16 container on loopback port 55495,
database `dex_indexer_issue1395`, and a unique test lock. Memory-backed database
storage removed host disk contention; all input identities were synthetic.
`indexer/.env` stayed ignored. A temporary Cargo home/target separated builds;
indexer package debug info was disabled in the local unoptimized test profile.

```bash
CARGO_HOME=/tmp/issue1395-cargo \
CARGO_TARGET_DIR=/tmp/cl8y-dex-1395-target \
CARGO_NET_OFFLINE=true make verify-issue-1395
```

- **PASS:** 490 library tests, twice, including typed parsing, zero/optional policy,
  oversized strings, 10,000 poisoned action headers, provenance, column alignment,
  fee identity, rounding and bounded diagnostics.
- **PASS:** nine `indexer_event_amount_integrity` Postgres scenarios, twice.
  Reconcile swaps/fees/positions/liquidity/hooks and daily evidence/volume/fee APIs;
  malformed blocks advance; infrastructure failures retain checkpoints; replay is
  deduplicated; real recovery script removes orphan fees; canonical replacement
  and position rebuild reconcile. Largest raw values, 18-decimal metadata,
  76-digit raw P&L, uncapped aggregate sums and legacy fractional evidence survive.
- **PASS:** 95 related tests: ingestion hardening 11, protocol fees 26, evidence API
  11, protocol volume 19, pair USD volume 6, positions 5, trader raw volumes 14,
  swap/fill linkage 2 and parked lifecycle 1.
- **PASS:** all 58 forward migrations applied in a separate isolated scratch DB
  using `psql -X -v ON_ERROR_STOP=1`; reversible `.down.sql` files were excluded.
- **PASS:** both updated #1277 source/documentation check blocks, shell syntax,
  new documentation link targets and `git diff --check`.
- **PASS:** normal `.githooks/pre-commit`: contract formatting, contract Clippy
  and secret scan. Pinned dependencies unavailable in the initial offline cache
  were fetched before the successful check. Commit hooks run again on staged work.

Initial failures were corrected and retested: noncanonical fractional raw fixture,
concurrent shared-cache tests (serialized), Rust fixture typing/shadowing, evidence
row ordering and reserve seeding from a snapshot instead of persisted event reserves.
No failed assertion was relabeled as a skip.

## Acceptance status and scope

- [x] **AC1:** malformed mandatory/present optional quantities reject their action;
  genuine zero and absent optional values retain schema semantics.
- [x] **AC2:** typed bounded diagnostics omit values/identities; poison replays and
  valid siblings preserve progress without a poisoned-row retry loop.
- [x] **AC3:** supported maximum quantities, decimals, multihop identities and
  unchanged-input replay remain exact; aggregate and P&L bounds reconcile.
- [x] **Database/API:** ledgers, rollups, evidence, retries and recovery checked.
- [ ] **Deployment — UNVERIFIED:** production deployment was outside this PR task;
  local tests do not establish deployed behavior.
- [ ] **Historical repair — NOT PERFORMED:** stored legacy zero alone cannot identify
  malformed source input. Repair requires original authenticated input and a
  separately scoped replay/rebuild.

The existing LP/hook occurrence-key coalescing limitation is documented in the
[checklist](README.md). This change does not add atomic block transactions or
certify historical accounting. The [agent playbook](../../../skills/AGENTS_INDEXER_EVENT_AMOUNTS.md)
and [A1395 invariants](../../indexer-invariants.md#exact-event-amounts-1395) define
reproduction. Agents may report overall PASS with explicit unavailable skips as
described in the checklist; leave skipped criteria unchecked and fix executed
failures before retesting.
