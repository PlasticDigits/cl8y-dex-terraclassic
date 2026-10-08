# Community invoice fee QA (#1210 / #1354)

[Source implementation #1354](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1354) adds separate `sku_unlock` and `settings_fee` sources to the governed ledger. #1210 tracking was reopened after its earlier docs-only close. See [verification](../issue-1354/verification.md) and [L1210-1–L1210-8](../../../skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md).

## Agent instructions

1. Read issue/comments and fee-ledger/provenance/DB1396 invariants. Provision disposable QA roles; never migrate the shared QA control DB.
2. Run `make verify-issue-1210`. It checks reserved-emitter action segmentation, emitted invoice/create SKU amounts, replay, API/UTC series/catalog and fixed UI labels.
3. Confirm token invoice provenance includes an anchored launcher receipt, expected token code/admin/CMM and exact matching token/launcher UST1 and treasury configuration. Origin self-report alone and unanchored adopted tokens fail closed.
4. Keep `make verify-issue-586`, `make verify-issue-614`, `make verify-issue-594` and #1269 regressions green. Capture a paid chain receipt and replay it into the new ledger when actual chain/custody is available.

Unavailable checks may be **SKIP** with reason/missing evidence and overall **PASS WITH SKIPS** when available checks pass. Executed failures remain FAIL. Shared harness options are `VERIFY_ISSUE_1210_SKIP_DB_REASON` / `_SKIP_UI_REASON`. A fixture or skipped deployment does not establish production paid-event acceptance.

## Acceptance

- [x] AC1: separate source/schema/rollup/daily/DeFiLlama/UI wiring.
- [x] AC2–AC4: emitted token enable/settings and create-time unique SKU count, no launcher/ready/per-field duplicate; zero creates excluded (fixtures).
- [x] AC5/AC6: reserved emitter/pins and paid provenance; no migrate-adopt/tax/gas/swap fee inference; malformed/no-op/reverted receipts rejected.
- [x] AC7: idle/unpriced/invalid-window and allowlisted labels; authenticated historical paid rows survive later pin removal.
- [x] AC8: pair-creation and catalog regression, widened pair-aware/non-pair replay keys; no client analytics changes.
- [x] Actual paid LocalTerra SKU receipt replayed through block ingestion into disposable QA DB twice; transport failure/retry, exact UST1 amount and dedupe verified.
- [ ] Production paid invoice history replay; no actual settings-batch receipt was available (synthetic fixture coverage passes).
- [ ] Production rollout with actual configured launcher/code/admin/UST1/CMM pins.

Catalog #594 remains lifecycle data. Fee ledger home #1213 is a design map; cohort #1211 stays separate. Unanchored legacy adopted-token invoice recovery needs trustworthy provenance before admission and is not inferred from enabled SKU flags.
