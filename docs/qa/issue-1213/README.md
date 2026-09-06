# QA — Forgejo #1213 fee-ledger home

Verify (no chain): `make verify-issue-1213`

Playbook: [`skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md`](../../../skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md) · invariants **L1213-1–L1213-8**.

This epic does **not** implement `FeeSource` parsers. Ingest lives on [#1209](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1209) / [#1210](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1210) / [#1211](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1211).

## Automated

- Skill names the three children and closed marketing trackers #1 / #4 / #5
- Invariants **L1213-1–L1213-8** present
- `docs/indexer-invariants.md` has a **Fee-ledger home #1213** row
- Factory audit item 11 points at #1209 (discovery ≠ treasury census)
- AGENTS.md + testing.md + wrap/window/catalog skills cross-link the home
- No new `pair_creation` / `sku_unlock` / `settings_fee` CHECK or `FeeSource` on this epic

## Manual / operator (not this verify)

1. Pair-creation ingest → implement **#1209**, not marketing #1.
2. SKU/settings invoices → **#1210**, not marketing #4.
3. Cohort split → **#1211**, not marketing #5.
4. Do **not** reopen the closed marketing trackers for implement work.

## Out of scope here

- New `FeeSource`, API field, or UI label.
- [#1204](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1204) OpenAPI pack.
- [#1202](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1202) client events.
- Social / X / paid media.
