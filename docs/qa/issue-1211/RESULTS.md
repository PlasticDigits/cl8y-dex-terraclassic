# Local verification: #1211 (2026-10-18)

Scope: source and synthetic provider/receipt/API fixtures on disposable isolated
Postgres. Build and test steps were separate; each actual test process had a
59-second ceiling. Servers undergoing upgrades were not reset.

| Focused binary/filter | Result | Test duration |
|---|---|---|
| `cl8y_dex_indexer fee_cohorts::tests` | 2 passed | 0.00s |
| `indexer_fee_cohorts` | 10 passed | 2.72s |
| `indexer_protocol_fees` | 31 passed | 8.45s |
| `indexer_tier_sync` | 4 passed | 1.01s |
| `indexer_economic_fee_usd` | 8 passed | 2.50s |
| `indexer_defillama` | 17 passed | 5.85s |

The cohort binary includes nine feature tests and one shared QA safety test.
The old tier-sync route test expected request-triggered remote optimization and
failed with the current local quote policy. Its replacement verifies the intended
current behavior: indexed live tier state alone does not authorize an unobserved
quote, the typed result is `missing_observations`, and the request performs zero
LCD calls. Tier registration indexing and reconciliation tests remain intact.

- [x] AC1 implementation: private payer/unjoined and immutable cohort persistence.
- [x] AC2 implementation: additive rollups for each allowed window; priced share, unpriced activity and idle semantics.
- [x] AC3 implementation: feature JSON contains aggregate fields only.
- [x] AC4 fixtures: historical unregistered tier 0 is retail; current mutable state cannot override it.
- [x] AC5 fixtures: same-transaction register/deregister ordering and immutable replay.
- [x] AC6 fixtures: tier 5/255 retail; MM taker follows parent swap rather than retail maker.
- [x] AC7 fixtures: explicit trader on a pinned router is attributed; missing/ambiguous/spoofed source actors remain unknown.
- [x] AC8 implementation: atomic consistent scheduled publication and bounded cached GET; source enum and client analytics unchanged.
- [x] Malformed registry transitions fail closed; ignored historical height cannot certify membership.
- [x] Failed cohort publication rolls back source/cohort changes; aggregate overflow matches source null semantics.
- [x] Actual LocalTerra direct-swap → retail, register tier 0 → genuine paid wrap → MM, production ingestion, exact H−1 LCD state and public HTTP privacy; [receipt](localterra-ingestion.json).
- [ ] Production schema migration and deployed indexer revision/readback.
- [ ] Live fee windows, exact-height provider behavior and bounded deployment logs.

Current contract receipts lack some payer attributes, as described in the
[checklist](README.md#actual-chain-limitations). Synthetic future-compatible
receipt shapes do not prove deployed emissions. Overall local tests pass; the
issue remains open pending its actual-chain/deployment criteria.

## Plain decimal HTTP follow-up

An actual HTTP dust fixture first reproduced `amount_usd: "1.00E-18"`
instead of the required `"0.000000000000000001"` (one test failed in 0.63s).
The new cohort amount/share serialization now uses the existing #557 plain
decimal helper. The fixture checks both 1e-18 USD and a 1e-18 percent share
through all three HTTP windows; all nine cohort checks pass in 2.45s afterward.
Only new `by_cohort` fields changed; existing source/token serialization remains
unchanged. Compilation was separate (40.51s); each actual test had a 59s ceiling.

## Actual owned LocalTerra acceptance

The combined actual-chain/ingestion run was bounded by `timeout 59s`.
The chain path took **1.98s** and the production-ingestion/public-API test **0.74s**.
It used an independent owned factory/pair/discount registry and genuine owned
ustr-cmm treasury/mapper/CW20 wrapping stack. No shared contract settings or
infrastructure were reset. Fresh actual REST LCD receipts were passed directly
through `parser::process_block_txs`; no fee rows, actor baselines or attribution
rows were manually inserted. Actual snapshot assertions certify H−1 unregistered
and registered tier-0 membership through the production LCD height guard.

- [x] An unregistered paid direct swap produced one priced `retail` fee.
- [x] Governance RegisterWallet tier 0 preceded a genuine native treasury wrap;
  its mapper fee produced one priced `mm` fee.
- [x] Replay after registration preserved the earlier retail classification.
- [x] Public `/protocol/fees?window=24h` returned only aggregate cohort keys and
  omitted the owned actor address.

USD uses explicitly labeled owned QA economic-mark/USTC-oracle fixtures; this
checks stamped-USD integration and does not claim a real external USD feed.
[The public receipt](localterra-ingestion.json) contains transaction linkage and
aggregate assertions; full actual synthetic-actor LCD receipts remain private in
`/tmp/dex-cohort-1211-receipts.json`.

This smoke exposed a real emission mismatch: current mapper `notify_deposit`
authenticates the treasury callback and emits the fee-paying `depositor`, while
the initial attributor expected `sender`. The fix accepts `depositor` only on the
same pinned canonical action, rejects duplicate depositor fields, and keeps
unwrap recipients unjoined. The focused regression passed with all ten cohort
checks. The owned fixture also initially hit the expected F6 code-ID guard:
matching CW20 bytes can have different code IDs; listing both actual owned IDs
in the owned factory completed provisioning without changing shared contracts.

## Compatibility-first local verification

The [compatibility checklist](compatibility.md) and [executed local receipt](compatibility-local-results.json)
record a serving-only 67/68 version pair with unchanged runtime privilege checks.
The focused wave passed 42 tests: one real-prefix owned HTTP/block-ingestion
transition (3.67 seconds including provisioning/cleanup), 10 cohort tests
(2.83 seconds) and 31 protocol-fee tests (9.10 seconds). Compiles were separate;
every locally invoked test process was bounded to 59 seconds. The initial fixture
mistakenly counted a down migration and stopped before applying migrations; its
owned container was removed and the corrected genuine 67-prefix run passed.

The final fixture checks actual 67 publisher overlap, typed pending schema/seed/mixed
clock states, coherent joint 68 publication, actual runtime login/startup guards
on both schemas, older/future rejection, protected runtime denials and unchanged
history/role attributes. This is a handler/startup-guard rehearsal, not a full
production process/native-quote or zero-downtime deployment attestation. The API
must remain serving; the prior stopped-runtime maintenance workflow is historical
and must not be repeated for this transition.
