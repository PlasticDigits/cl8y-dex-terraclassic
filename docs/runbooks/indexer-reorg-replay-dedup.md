# Indexer ingestion hardening — reorg, replay, dedup, and backfill

This runbook is for **operators** running [`indexer/`](../../indexer/). It complements [Indexer invariants](../indexer-invariants.md) and addresses **IX-03** (chain reorg / tx reorder) in [`docs/reviews/20260409T030009Z/SECURITY_REVIEW.md`](../reviews/20260409T030009Z/SECURITY_REVIEW.md).

Implementation: GitLab [**#236**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/236) (detection + cursor guard), [**#362**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/362) (operator alert + recovery preview). Agent playbook: [`skills/AGENTS_INDEXER_INGESTION_HARDENING.md`](../skills/AGENTS_INDEXER_INGESTION_HARDENING.md).

## Facts

- The indexer **polls the LCD** and advances a cursor stored as `last_indexed_height` plus **`last_indexed_block_hash`** in Postgres ([`indexer/src/db/queries/state.rs`](../../indexer/src/db/queries/state.rs)).
- **Automatic reorg detection:** before each new height, the poller re-fetches the hash at the last committed height; mismatch **halts** the indexer (no silent skip).
- **Block processing errors** do **not** advance the cursor; retries use `BLOCK_PROCESS_MAX_RETRIES` / `BLOCK_PROCESS_RETRY_BACKOFF_MS`; persistent failures are recorded in **`indexer_failed_blocks`**.

## Operator alert on halt (GitLab #362)

When a reorg is detected, the indexer emits:

1. A **`tracing` error** with target `indexer_reorg_halt` (route in your log collector).
2. A machine-parseable stderr line: `INDEXER_REORG_HALT {"event":"indexer_reorg_halt",...}` including `recovery_cmd` and `runbook`.
3. An optional **webhook POST** when `REORG_ALERT_WEBHOOK_URL` is set (PagerDuty, Slack incoming webhook, etc.). Payload is JSON with `height`, `stored_hash`, `canonical_hash`, and `recovery_cmd`.

Alert on the `INDEXER_REORG_HALT` prefix or `target=indexer_reorg_halt` in production. The indexer process exits the poller task on halt; restart only **after** cursor recovery.

## Dedup and replay

- **Swap dedup:** Inserts use a unique constraint on `(tx_hash, pair_id, swap_index)` with `ON CONFLICT DO NOTHING` ([`insert_swap`](../../indexer/src/db/queries/swap_events.rs); migration `20260605000000_swap_events_per_tx_pair_swap_index.sql`, GitLab [**#287**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/287)). `swap_index` is the per-pair ordinal within the tx so multiple genuine swaps on one pair are stored separately; re-processing the same block after a restart **skips** duplicate delivery of the same swap safely.
- **Replay:** Running the indexer again over heights that were already indexed is safe for idempotent rows (swaps); other tables (candles, positions, aggregates) are updated by merge logic—if you suspect corruption, treat as a **full re-backfill** from a known-good height (see below).

## Reorg handling

### Automatic detection

1. Indexer stores `(height, block_hash)` on each successful block ([`block_indexer::index_block`](../../indexer/src/indexer/block_indexer.rs)).
2. Before indexing `height + 1`, LCD block hash at `height` is compared to `last_indexed_block_hash`.
3. On mismatch: structured halt signal (above), process exit — **no further blocks indexed**.

### Shallow reorg (recommended path)

Use when the fork point is known and only a few blocks at the tip diverged.

| Step | Action |
|------|--------|
| 1 | Confirm halt: grep logs for `INDEXER_REORG_HALT` or `indexer_reorg_halt`. Note `height` and hashes from the JSON payload. |
| 2 | **Stop** the indexer (if not already halted). |
| 3 | Identify fork height `H` (first block to re-index on the new canonical chain; usually the mismatch height from the alert). |
| 4 | **Dry-run** recovery (row counts + cursor SQL preview): `make indexer-reorg-recover HEIGHT=H` or `./scripts/indexer-reorg-recover.sh --height H` |
| 5 | Review row-impact preview. Swaps at `block_height >= H` replay without duplicates; candles/trader aggregates may need manual cleanup if counts look wrong. |
| 6 | **Apply** cursor reset: `make indexer-reorg-recover HEIGHT=H APPLY=1` |
| 7 | **Restart** indexer; monitor until `last_indexed_height` catches LCD tip. |
| 8 | Spot-check API candles/trades vs LCD for affected pairs. |

Expected operator time on QA Postgres: **under 30 minutes** when fork depth is shallow and LCD is healthy.

### Deep reorg / corrupted derived state

Use when many blocks diverged, derived tables are inconsistent, or shallow replay leaves wrong candles/volumes.

1. **Detect:** Compare indexed tip with a **trusted** LCD / block explorer on the canonical chain.
2. **Stop** the indexer process.
3. **Choose recovery:**
   - **Restore Postgres** from a snapshot taken **before** the reorg window (preferred), **or**
   - Delete rows for affected heights and dependent aggregates (operationally heavy; use dry-run row counts as a guide).
4. **Reset cursor:** `make indexer-reorg-recover HEIGHT=H APPLY=1` or manual `indexer_state` update to **at least one block before** the fork point.
5. **Restart** the indexer and verify API consistency.

## Backfill

- **`START_BLOCK`:** Optional env ([`indexer/src/config.rs`](../../indexer/src/config.rs)). When `last_indexed_height` is `0`, the indexer can start after `START_BLOCK - 1`. Use only on a **fresh** or **cursor-reset** database.
- **Caution:** Backfilling from a mid-chain height **without** clearing inconsistent state can leave candles/traders wrong. Prefer a clean DB or a documented SQL cleanup plan.

## Config (ingestion + alerting)

| Env | Default | Purpose |
|-----|---------|---------|
| `BLOCK_TX_PAGE_LIMIT` | `100` | LCD `search_txs` page size per block |
| `BLOCK_TX_MAX_PAGES` | `50` | Max pages per block (bounds memory / pagination abuse) |
| `BLOCK_PROCESS_MAX_RETRIES` | `5` | Retries before halting on a failing block |
| `BLOCK_PROCESS_RETRY_BACKOFF_MS` | `2000` | Backoff base × attempt between retries |
| `REORG_ALERT_WEBHOOK_URL` | _(unset)_ | Optional webhook for reorg halt JSON (GitLab #362) |

## Related

- [Environment matrix](../environment-matrix.md) — LCD vs chain IDs.
- [Incident template](../templates/incident-dex-indexer.md) — escalation when indexer and chain diverge.
- [Indexer invariants — C1–C3](../indexer-invariants.md#indexing-invariants)
