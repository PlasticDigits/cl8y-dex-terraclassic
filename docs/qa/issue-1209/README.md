# Pair-creation protocol fee ingest (#1209)

**Source implemented through [#1354](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1354).** Creation-fee parsing, schema checks, ledger/replay, API/daily/DeFiLlama fields and UI labels are implemented. Live deployment and paid-chain replay remain separate evidence; #1209 tracking was reopened after its earlier docs-only close. See [#1354 verification](../issue-1354/verification.md).
## Required behavior

- Accept only `action=create_pair` emitted by the pinned `FACTORY_ADDRESS`, using the reserved `_contract_address` attribute. Ignore unreserved `contract_address` and spoofed emitters ([#285](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/285)).
- Insert at most one `pair_creation` row per successful create, only for positive `creation_fee_uluna`; use the emitted amount. Never count instantiate gas, fee-payer burn, overpay refunds, or `reply_instantiate_pair`. Do not also count the same treasury `BankMsg::Send`.
- Store the source as a non-pair fee (`pair_id IS NULL`) and inherit #1269's non-pair partial unique index. Keep `ON CONFLICT DO NOTHING`; never restore the old nullable three-column unique key or overwrite an existing amount.
- Price native `uluna` through the token catalog. Preserve the protocol-fee contract: event with no price remains present and `fee_usd = NULL`; idle configured source is `"0"`; all-unpriced activity is `null`; GET handlers continue using O(1) rollups and the `24h|7d|30d` allowlist.
- Wire additive fields through the fee rollups, `/api/v1/protocol/fees`, `/daily`, DeFiLlama, frontend `ProtocolFeeSourceKey`, and the fixed `Pair creation` label. Unknown DB source strings must not be rendered as labels.

## Verification needed before closing #1209

Run `make verify-issue-1209` against a DB1396 disposable QA cluster. It runs the shared invoice/creation parser, ledger/replay/API/series/catalog and allowlisted UI regressions. Keep #586/#614/#1269 regressions green. Factory audit item 11 now links the creation-fee parser.

- [x] AC1/AC8: source/schema/rollup/API/daily/DeFiLlama/UI and invariants/audit cross-links.
- [x] AC2–AC4: exact positive reserved-factory amount, no reply/gas/refund duplicate, zero/spoof rejected (fixtures).
- [x] AC5–AC7: idle/unpriced/invalid-window, fixed label safety and widened replay uniqueness (fixtures).
- [x] Actual paid LocalTerra creation receipt replayed through block ingestion into disposable QA DB twice, with exact native amount and dedupe.
- [ ] Production rollout and historical replay at actual configured factory.

Agents record UTC revision, commands and results. Unavailable checks may be **SKIP** with reason/missing evidence and overall **PASS WITH SKIPS** when available checks pass. Executed failures remain FAIL. Skip options: `VERIFY_ISSUE_1210_SKIP_DB_REASON` / `_SKIP_UI_REASON` (the shared harness). Never treat fixtures or a skipped rollout as production acceptance.

## Related work and agent guides

- [#1213](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1213) — closed fee-ledger home; assigns pair creation to this issue.
- [#1269](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269) — closed pair-aware AMM uniqueness; new non-pair sources use the NULL-pair partial index.
- [#1210](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1210) / [#1211](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1211) — later, separate invoice and cohort slices.
- [Factory treasury audit, item 11](../../audits/factory-treasury-bank-send.md), [indexer invariants](../../indexer-invariants.md), and [third-party fee-ledger playbook](../../../skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md).
