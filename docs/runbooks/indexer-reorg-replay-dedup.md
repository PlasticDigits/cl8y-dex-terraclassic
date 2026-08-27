# Indexer ingestion hardening — reorg, replay, dedup, and backfill

This runbook is for **operators** running [`indexer/`](../../indexer/). It complements [Indexer invariants](../indexer-invariants.md) and addresses **IX-03** (chain reorg / tx reorder) in [`docs/reviews/20260409T030009Z/SECURITY_REVIEW.md`](../reviews/20260409T030009Z/SECURITY_REVIEW.md).

Implementation: GitLab [**#236**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/236) (detection + cursor guard), [**#362**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/362) (operator alert + recovery preview). Agent playbook: [`skills/AGENTS_INDEXER_INGESTION_HARDENING.md`](../skills/AGENTS_INDEXER_INGESTION_HARDENING.md).

## Facts

- The indexer **polls the LCD** and advances a cursor stored as `last_indexed_height` plus **`last_indexed_block_hash`** in Postgres ([`indexer/src/db/queries/state.rs`](../../indexer/src/db/queries/state.rs)).
- **Automatic reorg detection:** before each new height, the poller re-fetches the hash at the last committed height; mismatch **halts** the indexer (no silent skip).
- **Operator alert on halt:** structured `tracing` event `indexer_reorg_halt` (target `indexer_reorg_halt`), stderr prefix `INDEXER_REORG_HALT`, plus optional webhook — see [Alerting](#alerting-on-reorg-halt).
- **Block processing errors** do **not** advance the cursor; retries use `BLOCK_PROCESS_MAX_RETRIES` / `BLOCK_PROCESS_RETRY_BACKOFF_MS`; persistent failures are recorded in **`indexer_failed_blocks`**.

## Dedup and replay

- **Swap dedup:** Inserts use a unique constraint on `(tx_hash, pair_id, swap_index)` with `ON CONFLICT DO NOTHING` ([`insert_swap`](../../indexer/src/db/queries/swap_events.rs); migration `20260605000000_swap_events_per_tx_pair_swap_index.sql`, GitLab [**#287**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/287)). `swap_index` is the per-pair ordinal within the tx so multiple genuine swaps on one pair are stored separately; re-processing the **same** canonical swaps after a restart **skips** duplicate delivery safely.
- **True reorg:** Canonical txs at affected heights may differ from what was indexed. Use `--cleanup-derived` before replay (see [Shallow reorg recovery](#shallow-reorg-recovery-1–few-blocks)).

## Alerting on reorg halt

When the hash guard fires, [`reorg_alert::emit_reorg_halt`](../../indexer/src/indexer/reorg_alert.rs) emits:

| Signal | Content |
|--------|---------|
| **Structured log** | `event=indexer_reorg_halt`, `height`, `stored_hash`, `canonical_hash`, `recovery_fork_height`, `recovery_command`, `runbook` |
| **Stderr JSON** | `INDEXER_REORG_HALT {"event":"indexer_reorg_halt",...}` including `recovery_command` and `runbook` |
| **Webhook** (optional) | JSON POST to `REORG_ALERT_WEBHOOK_URL` (PagerDuty Events, Slack incoming webhook, etc.) |

Wire log collectors to alert on `INDEXER_REORG_HALT` prefix, `event="indexer_reorg_halt"`, or `target="indexer_reorg_halt"`.

Example recovery command in the payload:

```bash
./scripts/indexer-reorg-recover.sh --height <FORK_HEIGHT>
```

## Reorg handling

### Automatic detection

1. Indexer stores `(height, block_hash)` on each successful block ([`block_indexer::index_block`](../../indexer/src/indexer/block_indexer.rs)).
2. Before indexing `height + 1`, LCD block hash at the **database** checkpoint height is compared to the hash stored with that height.
3. On mismatch: structured alert + process exit — **no further blocks indexed**.
4. If `last_indexed_height` already moved (second indexer during a Coolify rebuild), the poller **resyncs** instead of halting. Only one poller ingests per database (Postgres advisory lock).

### False halt: overlapping rebuild (not a chain fork)

Logs like `stored_hash` = canonical hash of **H+1** and `canonical_hash` = hash of **H**, with the process restarting every few seconds, mean two pollers overlapped. Do **not** run `indexer-reorg-recover.sh` for that pattern. Confirm LCD `/blocks/H` matches `canonical_hash`; if the indexer later indexed newer blocks, the cursor is consistent. Redeploy with a single replica (or this lock) so the new container waits instead of crash-looping.

### Shallow reorg recovery (1–few blocks)

Use when the fork point is known (alert log shows mismatch height) and Postgres snapshot restore is unnecessary.

1. **Stop** the indexer (if not already halted).
2. **Dry-run** recovery — review row-impact preview:

   ```bash
   ./scripts/indexer-reorg-recover.sh --height H
   ./scripts/indexer-reorg-recover.sh --height H --cleanup-derived
   ```

   Or via Make: `make indexer-reorg-recover HEIGHT=H` (add `CLEANUP=1` / `APPLY=1` when ready).

3. **Apply** cursor reset (+ derived cleanup when canonical txs changed):

   ```bash
   ./scripts/indexer-reorg-recover.sh --height H --cleanup-derived --apply
   ```

   - Sets `last_indexed_height` to `H - 1`, clears `last_indexed_block_hash`, truncates `indexer_failed_blocks`.
   - With `--cleanup-derived`: deletes `swap_events` and other block-height tables for `block_height >= H`, plus `candles` for affected pairs.

4. **Restart** the indexer; confirm `last_indexed_height` advances and API pair/candle data matches LCD tip within normal lag.
5. **Verify** swap count stable on replay (dedup) when **not** using `--cleanup-derived` and txs are unchanged; after cleanup, swaps are re-ingested fresh.

### Deep reorg recovery (many blocks / uncertain fork)

1. **Detect:** Compare indexed tip with a **trusted** LCD / block explorer on the canonical chain.
2. **Stop** the indexer process.
3. **Choose recovery:**
   - **Preferred:** Restore Postgres from a snapshot taken **before** the reorg window, then restart indexer, **or**
   - **Heavy SQL:** Use `--cleanup-derived` from an earlier fork height `H` and accept that `trader_positions` / trader rollups may need a full re-backfill or snapshot (not height-keyed). After the indexer is back, run **`cl8y-dex-indexer rebuild-positions`** (or wait for poller `repair_positions_if_trade_count_mismatch` — GitLab **#676**, `NUMERIC(78, 18)`). Block replay alone does not rebuild positions for swaps that already exist.
4. **Reset cursor** if not restoring snapshot: `last_indexed_height` to **at least one block before** the fork point (recovery script or manual `indexer_state` update).
5. **Restart** and monitor `tracing` logs.

### Halt → healthy catch-up checklist

| Step | Action | Pass criterion |
|------|--------|----------------|
| 1 | Indexer stopped | No running poller process |
| 2 | Fork height `H` identified | Matches alert `height` / LCD common ancestor |
| 3 | Dry-run script | Row-impact preview reviewed |
| 4 | `--apply` (+ `--cleanup-derived` if reorg) | `last_indexed_height` = `H - 1` |
| 5 | Restart indexer | Logs show blocks indexing from `H` |
| 6 | API spot-check | `/api/v1/pairs`, candles consistent with chain |

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
| `REORG_ALERT_WEBHOOK_URL` | *(unset)* | Optional webhook POST on reorg halt (**#362**) |

## Related

- [Environment matrix](../environment-matrix.md) — LCD vs chain IDs.
- [Incident template](../templates/incident-dex-indexer.md) — escalation when indexer and chain diverge.
- [Indexer invariants — C1–C3](../indexer-invariants.md#indexing-invariants)
- [Operator secrets](../operator-secrets.md) — log collectors, webhook URLs.
