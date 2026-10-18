# Fee actor and registry cohort verification (#1211)

Issue: [#1211](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1211).
Contract: [FC1211-1–FC1211-14](../../../skills/AGENTS_FEE_COHORTS.md).
Related: [fee-ledger home](../../../skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md),
[source provenance](../issue-1210/README.md),
[database isolation](../../../skills/AGENTS_INDEXER_DATABASE_ISOLATION.md),
[native/provider deadline ownership](../../../skills/AGENTS_NATIVE_QUOTE_RECOVERY.md).

## Implementation scope

Attribution runs after authenticated economic receipts are inserted. The private
1:1 attribution table stores a validated payer or explicit unjoined state and an
immutable cohort. Historical membership uses provider-certified H−1 state plus
pinned registry actions in chain order, rather than current `traders` state.
No registry history is fabricated when a provider lacks exact-height queries. Malformed pinned registry transitions cannot fall
back to prior membership; missing wallet identity creates a conservative unknown barrier.
Four reads and a shared two-second deadline bound additional provider work per
block; failures remain unknown. Ledger amounts are independent of these reads.

Three materialized aggregate rows per window supply `by_cohort` after a joint
published generation. [Compatibility serving](compatibility.md) keeps the API
active on 67 with cohorts explicitly pending and gates all new table access. All fee
breakdowns publish atomically from a consistent repeatable-read snapshot; GET
reads only these rollups in a single read snapshot and retains the 60-second
cache. Existing source/token payloads remain additive.

[Executed local results](RESULTS.md) record 72 passing focused checks plus the separately executed actual LocalTerra ingestion/API case.

## Acceptance checklist

- [x] AC1: immutable actor-or-unjoined/as-of cohort persistence verified by source fixtures and actual LocalTerra production ingestion; deployed schema 68/joint publication verified separately.
- [x] AC2: live `24h`, `7d`, `30d` return three available cohort rows; source/cohort counts and priced USD reconcile, shares total 100%. Idle/unpriced/null/plain-decimal cases are fixture-tested.
- [x] AC3: source/actual-chain/public API checks show aggregate-only cohort JSON without payer, wallet, registry or tier identities.
- [x] AC4: certified historical unregistered/default-tier-0 classification is retail in provider fixtures and the actual LocalTerra swap.
- [x] AC5: registration → MM shown by an actual paid LocalTerra wrap; deregistration, same-transaction ordering and immutable history/replay covered by fixtures.
- [x] AC6: source/tier/taker-vs-maker fixtures establish self tiers 1–9 and tier 255 retail, with book fees attributed to the taker.
- [x] AC7: trusted-router and spoof/unjoinable cases covered by source fixtures; genuine mapper depositor handled in actual LocalTerra receipts. Missing payer/trader emissions remain unknown.
- [x] AC8: bounded materialized GET, consistent joint cache publication and unchanged client analytics/FeeSource verified by fixtures/source; all production windows reconcile. Optional cohort UI was not shipped.

[Live release and migration acceptance](../issues-1206-1512/live-release-20261018.md)
records exact backend `5b623a04f35bb6b44d3e64df0014e458cad653fc`, schema 68,
hosted CI 526/527 PASS and [the 14-request public API smoke](../issues-1206-1512/live-api-20261018.json).
Behavior-specific criteria use their required source/provider fixtures and actual
LocalTerra path; public production API/schema criteria use the live receipts.
The historical windows remain conservatively unknown (208/1,257/3,837 events),
not fabricated retail/MM attribution. No new funded mainnet transaction was needed.

The fixtures cover chain/action ordering, mutable-live-tier disagreement, replay,
exact-height refusal, ordinal/emitter spoofing, payer/taker rules, duplicate actor
keys, privacy and priced/unpriced/legacy/idle semantics. Their PASS is distinct
from the actual LocalTerra ingestion path and deployed schema/API PASS. Together
these satisfy the issue body's and comments' required acceptance; no extra
per-criterion mainnet signing test is implied.

## Actual-chain limitations

The actual pinned mapper `notify_deposit` receipt names the native fee payer as
`depositor`; attribution accepts this authenticated field and rejects duplicate
values. Mapper unwrap `recipient` is a beneficiary and cannot establish the
payer, so unwrap receipts without a payer field remain unknown.

The current pair swap receipt emits `sender` without a `trader` attribute; routed
swaps lacking an authenticated original trader therefore remain unknown. Current
factory `create_pair` and community invoice receipts omit payer `sender`; those
receipts likewise remain unknown. A manager/receiver is not a substitute for the
fee payer. Synthetic fixtures that contain these attributes verify future
compatible receipts and cannot establish that the deployed contracts emit them.

The [owned LocalTerra receipt](localterra-ingestion.json) proves an actual paid
direct swap → retail, registry tier-0 registration → paid wrap → MM, exact H−1
provider queries, replay and aggregate-only public HTTP through production
ingestion. Explicit synthetic QA prices are not real USD feed acceptance.
Production migration, deployment and aggregate API acceptance are now recorded in
[the live release receipt](../issues-1206-1512/live-release-20261018.md).
The actual LocalTerra path supplies the required retail→MM behavioral proof;
production historical membership was not guessed or backfilled. Contract receipts
without authenticated payer/trader fields continue to be unknown by design.

## Commands

```sh
make verify-issue-1211
```

The verifier builds test binaries separately, then runs each relevant binary with
a 59-second ceiling. Each integration binary owns its disposable database; no
shared or production database is reset. Use `indexer/.env.qa`, never runtime DB
credentials. If compiling manually, retain the same `CARGO_TARGET_DIR` across
build/run steps and run the named executables emitted by Cargo.

For deployment, follow the [compatibility-first online sequence](online-migration.md):
first deploy and verify the exact released server supporting schema 67/68 while
the API remains serving. Then run the separate additive online migrator with its
private owner credential; the runtime never receives it. Require useful serving
health before and after the concurrent snapshot, preserved history/roles/ACLs,
and verified cleanup. After joint schema-68 publication, check the exact `/health`
SHA, all three cohort windows and a bounded logs.cl8y.com deployment interval.
Compare cohort totals with source totals without exposing actors. Do not repeat
the superseded stop-serving/migrate-first procedure. No wallet signing is needed.

Production schema preparation and isolated SELECT-only evidence are documented in
[deployment.md](deployment.md). This preflight does not apply the migration or
establish deployed cohort acceptance.

## Reproducing the owned LocalTerra path

Prepare `scripts/qa/localterra-cohort-1211.py prepare` separately from actual
acceptance. It reuses the owned #1229 manager and exact reviewed wasm hashes,
creates independent contracts/liquidity and does not reset shared infrastructure.
Build `localterra_fee_cohorts_1211` with `cargo test --no-run` before the timed
run. Under a single 59-second outer timeout, run the Python helper's `run` mode,
then the emitted Rust test executable with `--ignored --test-threads=1`.

The Rust case uses a marked, process-owned disposable QA database and the real
local LCD. Fresh receipts keep historical queries within the chain's retained
state. Registration changes the owned registry, so another actual run requires
an explicitly reset/new owned registry before starting; never alter a shared
registry to make the case pass. Source/provider fixtures continue to verify
missing or uncertified history fails closed.

Current deployment: [online migration and continuity checklist](online-migration.md).

Dated production follow-up: [exact release, online migration and bounded live API evidence](../issues-1206-1512/live-release-20261018.md).
Historical local scope/failed receipts above are retained. The dated follow-up
records actual deployment separately and keeps remaining issue acceptance explicit.
