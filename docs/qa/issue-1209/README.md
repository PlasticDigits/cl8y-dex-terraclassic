# Pair-creation protocol fee ingest (#1209)

**Status on refreshed `origin/main` (`54c4868e`): pending.** The indexer still has seven `FeeSource` values, no `create_pair` fee parser, and no `make verify-issue-1209` target. Keep [Forgejo #1209](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1209) open; [#1213](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1213) is the closed home map, not a replacement implementation ticket.

## Required behavior

- Accept only `action=create_pair` emitted by the pinned `FACTORY_ADDRESS`, using the reserved `_contract_address` attribute. Ignore unreserved `contract_address` and spoofed emitters ([#285](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/285)).
- Insert at most one `pair_creation` row per successful create, only for positive `creation_fee_uluna`; use the emitted amount. Never count instantiate gas, fee-payer burn, overpay refunds, or `reply_instantiate_pair`. Do not also count the same treasury `BankMsg::Send`.
- Store the source as a non-pair fee (`pair_id IS NULL`) and inherit #1269's non-pair partial unique index. Keep `ON CONFLICT DO NOTHING`; never restore the old nullable three-column unique key or overwrite an existing amount.
- Price native `uluna` through the token catalog. Preserve the protocol-fee contract: event with no price remains present and `fee_usd = NULL`; idle configured source is `"0"`; all-unpriced activity is `null`; GET handlers continue using O(1) rollups and the `24h|7d|30d` allowlist.
- Wire additive fields through the fee rollups, `/api/v1/protocol/fees`, `/daily`, DeFiLlama, frontend `ProtocolFeeSourceKey`, and the fixed `Pair creation` label. Unknown DB source strings must not be rendered as labels.

## Verification needed before closing #1209

Add focused parser/API/frontend coverage and `make verify-issue-1209`. Cover positive/zero amount, pinned and spoofed emitters, flattened wasm segments, refunds/gas/reply exclusions, replay deduplication, price/idle semantics, daily + DeFiLlama output, and UI label safety. Keep [#586](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586), [#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614), and [#1269](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269) regressions green. Update factory audit item 11 after the parser exists.

## Related work and agent guides

- [#1213](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1213) — closed fee-ledger home; assigns pair creation to this issue.
- [#1269](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269) — closed pair-aware AMM uniqueness; new non-pair sources use the NULL-pair partial index.
- [#1210](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1210) / [#1211](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1211) — later, separate invoice and cohort slices.
- [Factory treasury audit, item 11](../../audits/factory-treasury-bank-send.md), [indexer invariants](../../indexer-invariants.md), and [third-party fee-ledger playbook](../../../skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md).
