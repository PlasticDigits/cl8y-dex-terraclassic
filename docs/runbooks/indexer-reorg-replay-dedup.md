# Runbook: indexer reorg, replay, dedup, and backfill

This runbook is for **operators** running [`indexer/`](../../indexer/). It complements [Indexer invariants](../indexer-invariants.md) and addresses **IX-03** (chain reorg / tx reorder) in [`docs/reviews/20260409T030009Z/SECURITY_REVIEW.md`](../reviews/20260409T030009Z/SECURITY_REVIEW.md).

## Facts

- The indexer **polls the LCD** and advances a cursor stored as `last_indexed_height` in Postgres ([`indexer/src/db/queries/state.rs`](../../indexer/src/db/queries/state.rs)).
- There is **no automatic reorg detection** or chain rewind in the indexer. A deep reorg on the canonical chain requires **manual** recovery.

## Dedup and replay

- **Swap dedup:** Inserts use a unique constraint on `(tx_hash, pair_id)` with `ON CONFLICT DO NOTHING` ([`insert_swap`](../../indexer/src/db/queries/swap_events.rs)). Re-processing the same block after a restart **skips** duplicate swaps safely.
- **Replay:** Running the indexer again over heights that were already indexed is safe for idempotent rows (swaps); other tables (candles, positions, aggregates) are updated by merge logic—if you suspect corruption, treat as a **full re-backfill** from a known-good height (see below).

## Reorg handling (manual)

1. **Detect:** Compare your indexed tip (`last_indexed_height`) with a **trusted** LCD / block explorer on the canonical chain. If the chain reorganized below your tip, rows may reference txs that are no longer canonical.
2. **Stop** the indexer process.
3. **Choose recovery:**
   - **Restore Postgres** from a snapshot taken **before** the reorg window, **or** delete rows for affected heights and dependent aggregates (operationally heavy; script with care).
4. **Reset cursor:** Set `last_indexed_height` in `indexer_state` to **at least one block before** the fork point (or use `START_BLOCK` / empty DB strategy per your deployment).
5. **Restart** the indexer and monitor logs (`tracing`); optional `GET /metrics` on the **dedicated** metrics address if `METRICS_BIND` is set ([`docs/operator-secrets.md`](../operator-secrets.md), GitLab [#125](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/125)).

## Backfill

- **`START_BLOCK`:** Optional env ([`indexer/src/config.rs`](../../indexer/src/config.rs)). When `last_indexed_height` is `0`, the indexer can start after `START_BLOCK - 1`. Use only on a **fresh** or **cursor-reset** database.
- **Caution:** Backfilling from a mid-chain height **without** clearing inconsistent state can leave candles/traders wrong. Prefer a clean DB or a documented SQL cleanup plan.

## Related

- [Environment matrix](../environment-matrix.md) — LCD vs chain IDs.
- [Incident template](../templates/incident-dex-indexer.md) — escalation when indexer and chain diverge.
