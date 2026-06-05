# Indexer ingestion hardening — reorg, replay, dedup, and backfill

This runbook is for **operators** running [`indexer/`](../../indexer/). It complements [Indexer invariants](../indexer-invariants.md) and addresses **IX-03** (chain reorg / tx reorder) in [`docs/reviews/20260409T030009Z/SECURITY_REVIEW.md`](../reviews/20260409T030009Z/SECURITY_REVIEW.md).

Implementation: GitLab [**#236**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/236). Agent playbook: [`skills/AGENTS_INDEXER_INGESTION_HARDENING.md`](../skills/AGENTS_INDEXER_INGESTION_HARDENING.md).

## Facts

- The indexer **polls the LCD** and advances a cursor stored as `last_indexed_height` plus **`last_indexed_block_hash`** in Postgres ([`indexer/src/db/queries/state.rs`](../../indexer/src/db/queries/state.rs)).
- **Automatic reorg detection:** before each new height, the poller re-fetches the hash at the last committed height; mismatch **halts** the indexer (no silent skip).
- **Block processing errors** do **not** advance the cursor; retries use `BLOCK_PROCESS_MAX_RETRIES` / `BLOCK_PROCESS_RETRY_BACKOFF_MS`; persistent failures are recorded in **`indexer_failed_blocks`**.

## Dedup and replay

- **Swap dedup:** Inserts use a unique constraint on `(tx_hash, pair_id, swap_index)` with `ON CONFLICT DO NOTHING` ([`insert_swap`](../../indexer/src/db/queries/swap_events.rs); migration `20260605000000_swap_events_per_tx_pair_swap_index.sql`, GitLab [**#287**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/287)). `swap_index` is the per-pair ordinal within the tx so multiple genuine swaps on one pair are stored separately; re-processing the same block after a restart **skips** duplicate delivery of the same swap safely.
- **Replay:** Running the indexer again over heights that were already indexed is safe for idempotent rows (swaps); other tables (candles, positions, aggregates) are updated by merge logic—if you suspect corruption, treat as a **full re-backfill** from a known-good height (see below).

## Reorg handling

### Automatic detection

1. Indexer stores `(height, block_hash)` on each successful block ([`block_indexer::index_block`](../../indexer/src/indexer/block_indexer.rs)).
2. Before indexing `height + 1`, LCD block hash at `height` is compared to `last_indexed_block_hash`.
3. On mismatch: structured error log, process exit — **no further blocks indexed**.

### Semi-automated recovery

1. **Stop** the indexer process (if not already halted).
2. Identify the **fork point** height `H` (last block common to old and new canonical chain).
3. Run cursor reset (dry-run first):

   ```bash
   ./scripts/indexer-reorg-recover.sh --height H
   ./scripts/indexer-reorg-recover.sh --height H --apply
   ```

   This sets `last_indexed_height` to `H - 1`, clears `last_indexed_block_hash`, and truncates `indexer_failed_blocks`.

4. **Derived tables:** Swaps dedupe on replay, but candles / positions / trader aggregates may need SQL cleanup for heights `>= H` if you did not restore Postgres from snapshot. See manual steps below when in doubt.
5. **Restart** the indexer and monitor logs (`tracing`); watch for block processing errors and timestamp fallback warnings — see [`docs/operator-secrets.md`](../operator-secrets.md) and [GitLab #200](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/200) (tracing-only observability).

### Manual recovery (deep reorg / no script)

1. **Detect:** Compare indexed tip with a **trusted** LCD / block explorer on the canonical chain.
2. **Stop** the indexer process.
3. **Choose recovery:**
   - **Restore Postgres** from a snapshot taken **before** the reorg window, **or** delete rows for affected heights and dependent aggregates (operationally heavy; script with care).
4. **Reset cursor:** Set `last_indexed_height` in `indexer_state` to **at least one block before** the fork point (or use `START_BLOCK` / empty DB strategy per your deployment).
5. **Restart** the indexer.

## Backfill

- **`START_BLOCK`:** Optional env ([`indexer/src/config.rs`](../../indexer/src/config.rs)). When `last_indexed_height` is `0`, the indexer can start after `START_BLOCK - 1`. Use only on a **fresh** or **cursor-reset** database.
- **Caution:** Backfilling from a mid-chain height **without** clearing inconsistent state can leave candles/traders wrong. Prefer a clean DB or a documented SQL cleanup plan.

## Config (ingestion)

| Env | Default | Purpose |
|-----|---------|---------|
| `BLOCK_TX_PAGE_LIMIT` | `100` | LCD `search_txs` page size per block |
| `BLOCK_TX_MAX_PAGES` | `50` | Max pages per block (bounds memory / pagination abuse) |
| `BLOCK_PROCESS_MAX_RETRIES` | `5` | Retries before halting on a failing block |
| `BLOCK_PROCESS_RETRY_BACKOFF_MS` | `2000` | Backoff base × attempt between retries |

## Related

- [Environment matrix](../environment-matrix.md) — LCD vs chain IDs.
- [Incident template](../templates/incident-dex-indexer.md) — escalation when indexer and chain diverge.
- [Indexer invariants — C1–C3](../indexer-invariants.md#indexing-invariants)
