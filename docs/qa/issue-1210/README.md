# QA — Forgejo #1210

**Status: open; do not close.** Reviewed 2026-09-24 in a worktree at `origin/main` (`54c4868e`). No newer issue replaces the SKU/settings protocol-fee ingest.

## Findings

- `FeeSource::ALL` has seven sources; neither `sku_unlock` nor `settings_fee` exists in the enum, source constraint, parser, fee rollups, DeFiLlama mapping, or frontend labels.
- `community_token_events` stores SKU/settings catalog events and invoice attrs, but `/api/v1/protocol/fees` does not read that table.
- `make verify-issue-1213` passes its four design/home checks; it explicitly validates the fee-ledger map and that no child ingest was implemented. It is not a #1210 acceptance test.
- No `make verify-issue-1210` target or paid transaction evidence exists in this checkout.

## Remaining work

1. Implement the two invoice sources with per-action parser fixtures: pinned launcher create-time `sku_count × 50e6`; token `EnableFeature.invoice`; flat `UpdateSettings.invoice`; one row per CMM UST1 invoice; reject duplicates, no-op/revert, malformed amounts, and unrelated tax/gas/swap/wrap events. Require trustworthy token origin; [#1229](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1229) remains open on launcher-origin spoofing.
2. Wire source checks, rollups, `/protocol/fees`, `/daily`, DeFiLlama, and UI labels while preserving O(1) / 60s GETs, omit-unconfigured, idle `"0"`, and unpriced `null` behavior.
3. Inherit the widened uniqueness from [#1269](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269), add the `make verify-issue-1210` regression bundle and catalog regressions, and capture a paid LocalTerra transaction in `protocol_fee_events` before closing.

## Related issue map

- [#1213](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1213) is the closed design/home epic; it does not implement ingest.
- [#1209](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1209) is the separate pair-creation source; [#1211](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1211) is the separate cohort split. Both remain open.
- [#594](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/594) is the community-token catalog, not fee truth. [#1237](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1237) / [#1239](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1239) fixed on-chain no-op invoice behavior; [#1269](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269) updated the fee uniqueness key. None replaces #1210.

The implementation contract and system cross-links for third-party agents are in [`skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md`](../../../skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md) (**L1210-1–L1210-8**).
