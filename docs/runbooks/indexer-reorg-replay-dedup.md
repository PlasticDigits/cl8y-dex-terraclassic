# Indexer ingestion hardening — reorg, replay, dedup, and backfill

This runbook is for **operators** running [`indexer/`](../../indexer/). It complements [Indexer invariants](../indexer-invariants.md) and addresses **IX-03** (chain reorg / tx reorder) in [`docs/reviews/20260409T030009Z/SECURITY_REVIEW.md`](../reviews/20260409T030009Z/SECURITY_REVIEW.md).

Implementation: GitLab [**#236**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/236), alerting + recovery automation [**#362**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/362). Agent playbook: [`skills/AGENTS_INDEXER_INGESTION_HARDENING.md`](../skills/AGENTS_INDEXER_INGESTION_HARDENING.md).

## Facts

- The indexer **polls the LCD** and advances a cursor stored as `last_indexed_height` plus **`last_indexed_block_hash`** in Postgres ([`indexer/src/db/queries/state.rs`](../../indexer/src/db/queries/state.rs)).
- **Automatic reorg detection:** before each new height, the poller re-fetches the hash at the last committed height; mismatch **halts** the indexer (no silent skip).
- **Block processing errors** do **not** advance the cursor; retries use `BLOCK_PROCESS_MAX_RETRIES` / `BLOCK_PROCESS_RETRY_BACKOFF_MS`; persistent failures are recorded in **`indexer_failed_blocks`**.

## Operator alert on reorg halt (#362)

When a hash mismatch halts the indexer, it emits a structured log event:

| Field | Value |
|-------|--------|
| `target` | `indexer::reorg_alert` |
| `event` | `indexer_reorg_halt` |
| `height` | Last committed height where stored hash ≠ canonical |
| `stored_hash` / `canonical_hash` | Mismatch detail |
| `operator_action` | Recovery script + runbook pointer |

Configure log collectors to page on `event=indexer_reorg_halt` or `INDEXER_REORG_HALT` in the message.

**Optional webhook** (PagerDuty, Slack incoming webhook, custom ops endpoint):

```bash
export REORG_ALERT_WEBHOOK_URL="https://hooks.example.com/indexer-reorg"
```

POST body (JSON): `event`, `height`, `stored_hash`, `canonical_hash`, `recovery_runbook`, `recovery_script`, `operator_action`. Delivery is best-effort and non-blocking.

## Dedup and replay

- **Swap dedup:** Inserts use a unique constraint on `(tx_hash, pair_id, swap_index)` with `ON CONFLICT DO NOTHING` ([`insert_swap`](../../indexer/src/db/queries/swap_events.rs); migration `20260605000000_swap_events_per_tx_pair_swap_index.sql`, GitLab [**#287**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/287)). `swap_index` is the per-pair ordinal within the tx so multiple genuine swaps on one pair are stored separately; re-processing the same block after a restart **skips** duplicate delivery of the same swap safely.
- **Replay:** Running the indexer again over heights that were already indexed is safe for idempotent rows (swaps); other tables (candles, positions, aggregates) are updated by merge logic—if you suspect corruption, treat as a **full re-backfill** from a known-good height (see below).

## Reorg handling

### Automatic detection

1. Indexer stores `(height, block_hash)` on each successful block ([`block_indexer::index_block`](../../indexer/src/indexer/block_indexer.rs)).
2. Before indexing `height + 1`, LCD block hash at `height` is compared to `last_indexed_block_hash`.
3. On mismatch: structured `indexer_reorg_halt` alert (log + optional webhook), process exit — **no further blocks indexed**.

### Shallow reorg (≤ few blocks at tip)

Use when the fork point is near the indexed tip and Postgres was not restored from snapshot.

1. **Stop** the indexer (if not already halted by the reorg guard).
2. **Identify fork height `H`** — last block common to the old and new canonical chain (compare LCD/explorer to `last_indexed_height` and hashes in logs).
3. **Dry-run recovery** (row-impact preview + SQL):

   ```bash
   ./scripts/indexer-reorg-recover.sh --height H
   # or: make indexer-reorg-recover HEIGHT=H
   ```

   Review per-table row counts at `block_height >= H` and the cursor-reset SQL.

4. **Apply** (requires explicit `--apply`):

   ```bash
   ./scripts/indexer-reorg-recover.sh --height H --apply
   ```

   Swaps replay safely via dedup. If limit-order rows or candles look wrong after catch-up, re-run with derived cleanup:

   ```bash
   ./scripts/indexer-reorg-recover.sh --height H --cleanup-derived --apply
   ```

5. **Restart** the indexer and monitor logs until `last_indexed_height` catches LCD tip (expected poll lag only).

**Expected downtime:** under 30 minutes on QA Postgres when fork depth is shallow and LCD is healthy.

### Deep reorg (many blocks / uncertain derived state)

Use when fork depth is large, derived aggregates may be corrupt, or shallow replay leaves inconsistent candles/trader stats.

1. **Detect:** Compare indexed tip with a **trusted** LCD / block explorer on the canonical chain; confirm `indexer_reorg_halt` in logs.
2. **Stop** the indexer process.
3. **Choose recovery:**
   - **Preferred:** Restore Postgres from a snapshot taken **before** the reorg window, then set cursor to snapshot height, **or**
   - **In-place:** `./scripts/indexer-reorg-recover.sh --height H --cleanup-derived --apply` to delete height-indexed rows and rewind cursor (review dry-run counts first).
4. **Aggregate tables** (`ohlcv_candles`, `traders`, `trader_positions`, `token_volume_stats`, `pair_volume_24h`, `global_stats_24h`, mirror snapshots) have no per-block rows — after deep cleanup, allow indexer catch-up and scheduled refresh cycles, or run documented SQL rebuild if API still diverges from LCD.
5. **Restart** the indexer from cursor `H - 1`.

### Manual recovery (no script)

Same as deep reorg step 3–5 without the script: hand-edit `indexer_state`, truncate `indexer_failed_blocks`, and run height-filtered `DELETE` on affected tables. Prefer the script for consistent dry-run preview.

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
| `REORG_ALERT_WEBHOOK_URL` | _(unset)_ | Optional POST on `indexer_reorg_halt` (#362) |

## Related

- [Environment matrix](../environment-matrix.md) — LCD vs chain IDs.
- [Incident template](../templates/incident-dex-indexer.md) — escalation when indexer and chain diverge.
- [Indexer invariants — C1–C3](../indexer-invariants.md#indexing-invariants)
