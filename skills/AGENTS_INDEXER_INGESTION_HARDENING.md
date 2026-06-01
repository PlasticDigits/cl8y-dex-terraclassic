# Agent playbook: indexer ingestion hardening (GitLab #236)

## When to use

You are changing **`indexer/src/indexer/poller.rs`**, **`block_indexer.rs`**, **`lcd/mod.rs` `get_block_txs`**, cursor state, or debugging **skipped blocks**, **truncated busy blocks**, or **reorg / hash mismatch** halts.

## Invariants (C1–C3)

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| **C1** | `last_indexed_height` advances **only** after successful block ingest | [`block_indexer::index_block`](../../indexer/src/indexer/block_indexer.rs); `process_block_txs` propagates errors |
| **C2** | All txs in a block ingested via LCD pagination | `LcdClient::get_block_txs` loops until `pagination.total` met |
| **C3** | Reorg at tip detected via **block hash** mismatch | `verify_checkpoint_unchanged` before each new height |

Parent gap analysis: [`gaps/GAP_1780200149.md`](../../gaps/GAP_1780200149.md). Human doc: [`docs/indexer-invariants.md`](../docs/indexer-invariants.md). Operator runbook: [`docs/runbooks/indexer-reorg-replay-dedup.md`](../docs/runbooks/indexer-reorg-replay-dedup.md).

## Key files

| Area | Path |
|------|------|
| Poller loop | [`indexer/src/indexer/poller.rs`](../../indexer/src/indexer/poller.rs) |
| Block ingest | [`indexer/src/indexer/block_indexer.rs`](../../indexer/src/indexer/block_indexer.rs) |
| LCD pagination | [`indexer/src/lcd/mod.rs`](../../indexer/src/lcd/mod.rs) |
| Cursor + failed blocks | [`indexer/src/db/queries/state.rs`](../../indexer/src/db/queries/state.rs) |
| Recovery script | [`scripts/indexer-reorg-recover.sh`](../../scripts/indexer-reorg-recover.sh) |
| Tests | [`indexer/tests/indexer_ingestion_hardening.rs`](../../indexer/tests/indexer_ingestion_hardening.rs), `lcd/mod.rs` wiremock unit tests |

## Tests

**Library (no Postgres):**

```bash
cd indexer && cargo test --lib lcd::tests indexer::block_indexer
```

**Integration (Postgres + migrations):**

```bash
docker compose up -d postgres
./scripts/setup-postgres-dev-databases.sh
cd indexer && cargo test --tests -j 1 -- --test-threads=1
```

Focused:

```bash
cd indexer && cargo test --test indexer_ingestion_hardening -j 1 -- --test-threads=1
```

If host `psql` to `127.0.0.1:5432` hangs (Docker pg_hba), run the test binary from the postgres network namespace — see [`AGENTS_LOCAL_POSTGRES_DEV.md`](./AGENTS_LOCAL_POSTGRES_DEV.md).

## Operator recovery

```bash
./scripts/indexer-reorg-recover.sh --height FORK_HEIGHT   # dry-run
./scripts/indexer-reorg-recover.sh --height FORK_HEIGHT --apply
# restart indexer
```

Query failed blocks: `SELECT * FROM indexer_failed_blocks ORDER BY height;`

## Do not regress

- Swap dedup `(tx_hash, pair_id)` on replay.
- **Merged wasm fill parsing:** hybrid multi-maker txs must produce one `limit_order_fills` row per on-chain `limit_order_fill` action even when LCD flattens the stream and `action=swap` is last ([#269](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/269), pattern [#141](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/141)).
- Existing integration test binaries under `indexer/tests/` (22+ files).
- Warn-only skip inside `process_block_txs` for hard failures — errors must propagate (**C1**).
