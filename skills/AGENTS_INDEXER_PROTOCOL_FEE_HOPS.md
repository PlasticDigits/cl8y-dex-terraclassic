# Agent playbook: persist every multihop AMM hop in protocol_fee_events (#1269)

Audience: third-party agents changing `protocol_fee_events` uniqueness, `swap_amm` ingest, fee backfill, or copying the unique key onto #1209 / #1210 / #1211 sources.

**Issue:** [Forgejo **#1269**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269)  
**Parent census:** [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) (**PFee-1–PFee-14**, [#586](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586))  
**Swap uniqueness precedent:** GitLab [#287](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/287) `(tx_hash, pair_id, swap_index)` on `swap_events`  
**Fee-ledger children:** [`AGENTS_INDEXER_FEE_LEDGER_HOME.md`](./AGENTS_INDEXER_FEE_LEDGER_HOME.md) (**L1213**) — inherit this **widened** key, do not copy `UNIQUE (tx_hash, source, ordinal)`  
**Invariants table:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Protocol fees #586 / #1269**)  
**ADR:** [`docs/adr/0005-protocol-fee-multihop-hops.md`](../docs/adr/0005-protocol-fee-multihop-hops.md) — uniqueness, backfill, alternatives, rollout  
**Overview:** [`docs/architecture.md`](../docs/architecture.md#indexer-protocol-fee-ledger)  
**Ops:** [`docs/runbooks/overview-global-stats-brin.md`](../docs/runbooks/overview-global-stats-brin.md) § Protocol fees; [`docs/runbooks/indexer-reorg-replay-dedup.md`](../docs/runbooks/indexer-reorg-replay-dedup.md)

## Problem class

`parse_swaps` assigns `swap_index` **per pair** (pair B restarts at 0). `#586` stored `swap_amm` fees with that index as a **per-tx** `ordinal` under `UNIQUE (tx_hash, source, ordinal)`. Hop 2+ in the same router tx collided; `ON CONFLICT DO NOTHING` kept the first hop. Volume rows were complete (`swap_events`); treasury census was not. Replay cannot heal: `trade_exists` returns before fee ingest unless the ingest path is also tried.

## Unique key (do not collapse)

| Source | `pair_id` | Unique index |
|--------|-----------|--------------|
| `swap_amm` | factory `pairs.id` (NOT NULL) | `(tx_hash, source, pair_id, ordinal)` WHERE `pair_id IS NOT NULL` |
| wrap / unwrap / ust1_* / book_take / limit_place | NULL | `(tx_hash, source, ordinal)` WHERE `pair_id IS NULL` |

PostgreSQL `UNIQUE` treats NULL as distinct. A nullable `pair_id` **without** these partials would double-count wrap/window on replay (**A6**). Do **not** `ON CONFLICT DO UPDATE`.

`ordinal` for `swap_amm` stays `swap.swap_index` (per-pair). Alignment with `swap_events` is the point — do not switch `swap_amm` to a global-in-tx counter on the same key.

## Invariants (F1269-1–F1269-8)

| ID | Rule |
|----|------|
| **F1269-1** | Router tx, two distinct factory pairs, both `commission_amount > 0`, both `swap_index == 0` → two `swap_amm` rows. Same-pair `swap_index` 0 then 1 → two rows. |
| **F1269-2** | Duplicate delivery of one hop inserts 0 rows. Never `DO UPDATE`. Amounts stay the first stored value. |
| **F1269-3** | Wrap / unwrap / ust1_mint / ust1_redeem / book_take / limit_place keep NULL `pair_id` and `(tx_hash, source, ordinal)` uniqueness. Two wrap ordinals in one tx still coexist; replay of ordinal 0 still dedups. |
| **F1269-4** | One-shot backfill from `swap_events.commission_amount > 0` plus poller startup. Do not rely on indexer replay alone. `fee_usd` NULL until the existing NULL-only stamp helper. Never rewrite non-null `fee_usd` (#568). |
| **F1269-5** | GET `/overview` and `/protocol/fees` stay O(1) rollup / 60s cache. Do not `SUM protocol_fee_events` on GET. Refresh rollup after backfill. |
| **F1269-6** | L7: hybrid still counts pool `commission_amount` + fill `commission_amount` once. Do not add `book_commission_amount`, spread, hook, burn tax, or gas to close a census gap. |
| **F1269-7** | #285: unreserved `contract_address` / undiscovered pair → no fee row. `amount_raw > 0` still required. Zero-commission hops: no fee row. |
| **F1269-8** | This skill + invariants + runbooks + `make verify-issue-1269`. Keep `make verify-issue-586` / `613` / `614` / `683`. #1209 / #1210 / #1211 must inherit the **widened** key. |

## Do / don’t

- **Do** pass `pair.id` into `FeeEventDraft` for `swap_amm`.
- **Do** try `ingest_swap_amm_fee` when `trade_exists` (idempotent heal).
- **Don’t** use a nullable unique without partial indexes / COALESCE sentinel.
- **Don’t** bind-mount `indexer/` into root Docker for cargo (`make test-indexer-target-ownership`).
- **Don’t** implement pair_creation / SKU / cohort here (#1209 / #1210 / #1211).
- **Don’t** keep two sqlx files on prefix `20260916120000` (hops vs [#1258](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1258) USDT). Keep first-applied USDT at `20260916120000`; hops is `20260916120001` (not `20260916120100`); see [ADR 0005](../docs/adr/0005-protocol-fee-multihop-hops.md) § Migration.

## Key files

| Area | Path |
|------|------|
| Migration | [`indexer/migrations/20260916120001_protocol_fee_events_pair_id.sql`](../indexer/migrations/20260916120001_protocol_fee_events_pair_id.sql) |
| Insert + backfill | [`indexer/src/db/queries/protocol_fees.rs`](../indexer/src/db/queries/protocol_fees.rs) |
| Ingest | [`indexer/src/indexer/parser.rs`](../indexer/src/indexer/parser.rs) `ingest_swap_amm_fee` |
| Draft | [`indexer/src/indexer/protocol_fees.rs`](../indexer/src/indexer/protocol_fees.rs) `FeeEventDraft.pair_id` |
| Startup | [`indexer/src/indexer/poller.rs`](../indexer/src/indexer/poller.rs) |
| Tests | [`indexer/tests/indexer_protocol_fees.rs`](../indexer/tests/indexer_protocol_fees.rs) |

## Regression

```bash
make setup-indexer-postgres
make verify-issue-1269
VERIFY_ISSUE_586_SKIP_E2E=1 VERIFY_ISSUE_586_SKIP_RELATED=1 make verify-issue-586
```

## Related

- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) — **PFee-14**
- [`AGENTS_INDEXER_FEE_LEDGER_HOME.md`](./AGENTS_INDEXER_FEE_LEDGER_HOME.md) — children inherit this key
- [`AGENTS_INDEXER_WRAP_FEE_INGEST.md`](./AGENTS_INDEXER_WRAP_FEE_INGEST.md) — wrap uniqueness unchanged
- [`AGENTS_INDEXER_UST1_WINDOW_FEES.md`](./AGENTS_INDEXER_UST1_WINDOW_FEES.md) — window uniqueness unchanged
- [`AGENTS_HOOK_COMMISSION.md`](./AGENTS_HOOK_COMMISSION.md) — L7 / **PFee-5**
