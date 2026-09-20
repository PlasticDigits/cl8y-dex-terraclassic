# ADR 0005: Trader rolling raw volume `NUMERIC(38, 0)`

## Status

Proposed ([#1277](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1277))

Ordinary leftover of [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548) raw-sum types. Not a [#297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297) authority exception. Not a founder card.

Versioned slices, tests, and rollout: [`design/1277-trader-rolling-volume-numeric.md`](../../design/1277-trader-rolling-volume-numeric.md). Playbook: [`skills/AGENTS_INDEXER_TRADER_ROLLING_VOLUME.md`](../../skills/AGENTS_INDEXER_TRADER_ROLLING_VOLUME.md).

## Context

`refresh_rolling_volumes` assigns `LEAST(SUM(offer_amount), 10^38-1)` into `traders.volume_24h` / `volume_7d` / `volume_30d`. Those columns (and lifetime `total_volume`) are still `NUMERIC(38, 18)` (`|x| < 10^20`). Sibling pair/global/token raw rollups already widened to `NUMERIC(38, 0)` in `20260817120000_*`. Positions/P&L use `NUMERIC(78, 18)` ([#676](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/676)); that type is inventory, not integer offer sums.

One 18-decimal sender in the 30d window fails the whole rolling `UPDATE`. Token/pair/global refresh in the same aggregator loop can still succeed; trader windows freeze. [#577](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/577) **D2** idle zero-out shares that transaction, so decay also stalls.

Widen-only is not Coolify-complete. `insert_swap` commits on the pool, then `update_trader_on_swap` → `upsert_trader` runs outside a wrapping transaction. On overflow the swap row stays; replay hits `trade_exists` / `inserted.is_none()` and never retries the trader upsert — the same skip class #676 repaired. `refresh_rolling_volumes` only `UPDATE`s existing `traders` rows.

## Decision

1. Widen `traders.volume_24h` / `volume_7d` / `volume_30d` / `total_volume` to **`NUMERIC(38, 0)`**. Keep `LEAST(…, POWER(10,38)-1)` on rolling sums. Cap `upsert_trader` `total_volume = traders.total_volume + $2` the same way. Leave `total_volume_usd` at `(38, 18)` with the existing `10^20` USD cap.
2. Same migration, after `ALTER TYPE`: **one-shot heal from `swap_events`**. `INSERT` missing senders. Recompute lifetime `total_trades` / `total_volume` with the integer cap. Do **not** rewrite `total_volume_usd` except via the existing [#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553) refresh.
3. Keep rolling refresh **UPDATE-only** plus the idle zero `UPDATE` (**D2**). Rely on the heal (migration + idempotent poller startup) so missing senders exist before the window stamp.
4. Unscoped trader JSON raw volumes use **`bd_plain_string`** (no `1e+19`).
5. Coolify: sqlx migrate + indexer restart. `traders` is small. Rollback is **not** `ALTER` back to `(38, 18)` once any value exceeds `10^20`.

## Invariants (V1277)

| ID | Rule |
|----|------|
| **V1277-1** | Rolling + lifetime raw volume columns are `NUMERIC(38, 0)`. Do not use #676 `NUMERIC(78, 18)` here. |
| **V1277-2** | Rolling `SUM` and additive `total_volume` use `LEAST(…, 10^38-1)`. Do not clamp raw volume to the USD `10^20` cap. |
| **V1277-3** | `total_volume_usd` stays `(38, 18)`. Heal does not touch it. Idle **D2** does not touch lifetime columns. |
| **V1277-4** | After overflow-skip, rebuild sender rows from `swap_events`. Block replay alone does not heal. |
| **V1277-5** | Rolling refresh stays UPDATE-only once heal has inserted senders. |
| **V1277-6** | Raw trader volume JSON is `bd_plain_string`. |
| **V1277-7** | `make verify-issue-1277` covers overflow refresh, decay, skip-heal, and unscoped board JSON. |
| **V1277-8** | Down-migration that narrows below stored integer width is forbidden. |

## Consequences

- Aggregator loop no longer 500s the trader stamp on ~100 human 18-dec tokens.
- Coolify leftovers (swap present, trader missing or undercounted) heal at migrate/startup, then rank on `sort=volume_24h`.
- Wrapping `insert_swap` + `upsert_trader` in one transaction is **out of scope** (would change poller error semantics for an overflow that this type change removes).
