# Local verification: #1211 (2026-10-18)

Scope: source and synthetic provider/receipt/API fixtures on disposable isolated
Postgres. Build and test steps were separate; each actual test process had a
59-second ceiling. Servers undergoing upgrades were not reset.

| Focused binary/filter | Result | Test duration |
|---|---|---|
| `cl8y_dex_indexer fee_cohorts::tests` | 2 passed | 0.00s |
| `indexer_fee_cohorts` | 8 passed | 2.29s |
| `indexer_protocol_fees` | 31 passed | 8.45s |
| `indexer_tier_sync` | 4 passed | 1.01s |
| `indexer_economic_fee_usd` | 8 passed | 2.50s |
| `indexer_defillama` | 17 passed | 5.85s |

The cohort binary includes seven feature tests and one shared QA safety test.
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
- [ ] Actual LocalTerra direct-swap → retail, register tier 0 → paid wrap/create → MM evidence.
- [ ] Production schema migration and deployed indexer revision/readback.
- [ ] Live fee windows, exact-height provider behavior and bounded deployment logs.

Current contract receipts lack some payer attributes, as described in the
[checklist](README.md#actual-chain-limitations). Synthetic future-compatible
receipt shapes do not prove deployed emissions. Overall local tests pass; the
issue remains open pending its actual-chain/deployment criteria.
