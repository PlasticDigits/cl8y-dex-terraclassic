# Fee actor and registry cohort verification (#1211)

Issue: [#1211](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1211).
Contract: [FC1211-1–FC1211-10](../../../skills/AGENTS_FEE_COHORTS.md).
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

Three materialized aggregate rows per window supply `by_cohort`. All fee
breakdowns publish atomically from a consistent repeatable-read snapshot; GET
reads only these rollups in a single read snapshot and retains the 60-second
cache. Existing source/token payloads remain additive.

[Executed local results](RESULTS.md) record 70 passing relevant checks.

## Acceptance checklist

- [ ] AC1: production ingested rows have an actor/unjoined and immutable as-of cohort.
- [ ] AC2: live `24h`, `7d`, `30d` return three additive cohort rows with priced/null/idle semantics.
- [ ] AC3: cohort JSON contains no payer, wallet, registry or tier identities.
- [ ] AC4: exact historical unregistered/default-tier-0 state classifies retail.
- [ ] AC5: register tier 0 → MM; deregister → retail; prior tags remain unchanged.
- [ ] AC6: self tiers 1–9 and tier 255 retail; book actor is taker, never maker.
- [ ] AC7: attributable trusted-router trader wins; unjoinable payer receipts are unknown.
- [ ] AC8: GET uses bounded rollups and one consistent cache snapshot; client analytics/FeeSource stay unchanged.

The integration fixtures cover chain/action ordering, mutable-live-tier disagreement,
replay immutability, exact-height success/refusal, parser ordinal spoofing,
source payer rules, custom events, duplicate actor keys, taker/maker, privacy,
priced/unpriced/legacy/idle state and allowed windows. Record executed results in
this task's release comment; unchecked boxes above require live receipts before closure.

## Actual-chain limitations

The current pair swap receipt emits `sender` without a `trader` attribute; routed
swaps lacking an authenticated original trader therefore remain unknown. Current
factory `create_pair` and community invoice receipts omit payer `sender`; those
receipts likewise remain unknown. A manager/receiver is not a substitute for the
fee payer. Synthetic fixtures that contain these attributes verify future
compatible receipts and cannot establish that the deployed contracts emit them.

No LocalTerra registration→paid wrap/create evidence, production migration,
deployment, or live provider acceptance is established by the local tests alone.
Servers undergoing upgrades are not reset by this work. Leave #1211 open until
its body and comments' runnable/live criteria have corresponding evidence.

## Commands

```sh
make verify-issue-1211
```

The verifier builds test binaries separately, then runs each relevant binary with
a 59-second ceiling. Each integration binary owns its disposable database; no
shared or production database is reset. Use `indexer/.env.qa`, never runtime DB
credentials. If compiling manually, retain the same `CARGO_TARGET_DIR` across
build/run steps and run the named executables emitted by Cargo.

For deployment: run the separate migrator with its private migration environment
first; do not start a new runtime against an unprepared schema. Use the existing
Coolify rollout for the reviewed commit. Check `/health` SHA and fees windows,
then inspect a bounded logs.cl8y.com interval for that deployment. Compare new
cohort totals with source totals and classify remaining unjoined receipts without
exposing actors. This is read-only after migration/deployment; it requires no
wallet signing.
