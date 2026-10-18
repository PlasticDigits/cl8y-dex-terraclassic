# Anonymous activity digest (#1206)

Applies to `GET /api/v1/evidence/digest?day=YYYY-MM-DD` and its aggregate SQL.
Issue: [#1206](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1206).

- AD1206-1: One strict nonfuture UTC calendar day, half-open midnight bounds. Reject duplicate/unknown parameters, CSV, identities, ranges and pagination. Today is incomplete; an empty day is 200.
- AD1206-2: Counts include all indexed markets, including gems. Parent swaps, LP add/remove and limit place/cancel/fill count independently. Never add book fills into swap counts.
- AD1206-3: Wrap/unwrap and window mint/redeem counts are treasury **fee-event counts**, not principal. Window counts never enter wrap.
- AD1206-4: No actor, wallet, transaction, pseudonym, unique-trader, pair breakdown or event tape. SQL may read only aggregate event/fee tables and protocol assets; never join events to people. Asset identities appear only on fee token rows; do not export arbitrary asset symbols.
- AD1206-5: Emit current `FeeSource::ALL` order, including idle sources. The issue's original seven became ten after #1209/#1210; retain the seven baseline entries and append pair_creation/sku_unlock/settings_fee. Unknown DB sources are excluded by a bound allowlist. Never freeze a stale seven-source enum.
- AD1206-6: Idle USD is "0"; any unpriced activity makes that source/token USD and share null. Headline preserves the sum of priced rows, null when all activity is unpriced. Shares use only priced USD; zero denominator is null. Plain decimal strings only.
- AD1206-7: Read-only repeatable snapshot, three aggregate queries, 700ms statement/200ms lock budgets, top eight assets plus other, shared two-export nonqueued admission and three-second HTTP owner. Apply the ordinary peer-IP governor; no LCD or request-triggered refresh.
- AD1206-8: Sanitized query/DB errors, no secret or raw-data logging. `/evidence/daily` remains a separate public-chain-linked event tape under [E1397](AGENTS_EVIDENCE_EXPORT_PRIVACY.md); it is not anonymous. Existing trailing fee/listing contracts remain unchanged.

Verification: [checklist](../docs/qa/issue-1206/README.md), `make verify-issue-1206`.
Build tests separately with `cargo test --locked --manifest-path indexer/Cargo.toml --test api_evidence_digest --no-run`; each actual test run has a 59-second limit. Follow [DB1396](AGENTS_INDEXER_DATABASE_ISOLATION.md) for disposable databases.
