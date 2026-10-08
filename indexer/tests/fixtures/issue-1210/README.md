# Paid LocalTerra receipt fixtures (#1209/#1210/#1354)

Captured read-only on 2026-10-08 from the already running LocalTerra LCD at
`127.0.0.1:1317`. No chain state or shared database was changed. These fixtures
prove emitted native creation and UST1 invoice amounts, not production deployment
or historical USD valuation. Full public event attributes are preserved.

- `localterra-paid-create-token.json`: successful tx
  `69E9D1029D53EB19385BB5722B04D045AED8B8EA3C460C5D67600645F3C2D498`,
  height 1707, one paid SKU, `sku_count=1`, 50,000,000 raw UST1 sent to CMM.
- `localterra-launcher-config.json`: read-only `get_config` from that exact
  reserved launcher, with the observed UST1, CMM treasury/governance and code ID.
- `localterra-paid-create-pair.json`: successful tx
  `BF76A3A06E91F7D693B04BCBA6D5E38F5743BB400110245781A5F296DDF563E3`,
  height 1722, `creation_fee_uluna=100000000`.

Reproduce the read-only capture with the LCD `GET /cosmos/tx/v1beta1/txs`
`query=wasm.action='create_token'` / `'create_pair'` (SDK 0.53 uses `query`, not
legacy `events`), and the pinned launcher's base64 `{"get_config":{}}` smart query.
Select successful receipts and preserve the reserved `_contract_address`.

`cargo test --test indexer_protocol_fees -- --test-threads=1` replays these exact
receipts through `process_block_txs` into the integration process's owned
throwaway database. The invoice test exercises an LCD outage before retry,
checks its row stays unpriced without a USD mark, and verifies replay uniqueness.
The factory test verifies one NULL-pair row and no treasury/gas/reply duplication.

See [agent fee-ledger instructions](../../../../skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md),
[QA #1209](../../../../docs/qa/issue-1209/README.md) and
[QA #1210](../../../../docs/qa/issue-1210/README.md).
