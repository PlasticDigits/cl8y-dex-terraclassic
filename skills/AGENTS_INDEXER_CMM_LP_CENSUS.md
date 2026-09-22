# Agent playbook: CMM-held v2 LP fee bps and volume rollover (#1317)

Audience: agents adding the three `/protocol` top-pairs columns or the stamps behind them.

**Issue:** [#1317](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1317)  
**ADR:** [`docs/adr/0012-cmm-held-v2-lp-fee-bps.md`](../docs/adr/0012-cmm-held-v2-lp-fee-bps.md) — formula, transfer gap, alternatives, rollout  
**Overview:** [`docs/architecture.md`](../docs/architecture.md#cmm-lp-census)  
**Ranking (do not reopen):** [`AGENTS_INDEXER_PROTOCOL_TOP_PAIRS.md`](./AGENTS_INDEXER_PROTOCOL_TOP_PAIRS.md) (**I1263**)  
**UI:** [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) (**P1317**)

## Invariants (I1317)

| ID | Rule |
|----|------|
| **I1317-1** | Custodian is `CMM_GOVERNANCE_ADDR` only. Unset or invalid → new JSON fields omitted. No second env and no hardcoded bech32. |
| **I1317-2** | `cmm_lp_usd = tw_shares / total_supply × pair_liquidity_usd`. Share-seconds over trailing 30 UTC days. Provide credits `receiver`, or `sender` when `receiver` is absent, by wasm `share`. Withdraw debits `sender` by `withdrawn_share`. LP-token `transfer` / `transfer_from` / `send` (match `pairs.lp_token` only) use `from`, `to`, and `amount` as `transfer_out` / `transfer_in`. Ignore `mint` and `burn`. Ignore a transfer or send when `from` or `to` is the pair contract. Chain idempotency is `ON CONFLICT (tx_hash, pair_id, kind, holder, event_index) DO NOTHING` with `kind` in `provide`, `withdraw`, `transfer_out`, `transfer_in`. `event_index` keeps two transfers of the same holder in one tx. Do not use `liquidity_event_exists`. The poller already walks every tx; no new subscription. |
| **I1317-3** | No anchor → NULL (not zero). The first successful custodian LCD read is a fixed anchor: `observed_at` is that block time, and `observed_height` is ≥ the last delta included in `shares`. Later reads, at most once per 24h and only for `CMM_GOVERNANCE_ADDR`, do not move `shares` or `observed_at`. They insert one `reconcile` delta at the later block time for `lcd_balance − (anchor.shares + deltas after the anchor)`. Do not replace `[window_start, latest_observed_at)` with the latest spot balance. Do not LCD every LP holder. Negative running balance clamps to 0. LCD is off the GET path. Regression, evaluated at day 30: 100 shares for 20 days then 500 for 10 days, with a refresh on day 29, stays `100×20/30 + 500×10/30`. A result of 500 fails. |
| **I1317-4** | `fees_usd` sums priced `swap_amm` + `book_take` + `limit_place` only. Wrap / unwrap / ust1_* excluded. Unpriced → NULL. Idle → `0`. `≥ 10^20` → NULL. |
| **I1317-5** | `book_take` / `limit_place` persist `pair_id` when the fill or placement match is unique. Wrap / window stay NULL. Never `DO UPDATE`. |
| **I1317-6** | `fees_bps = fees / cmm_lp × 10000` and `volume_per_cmm_lp = volume / cmm_lp`, same `≤ 0` / `≥ 10^20` omit as `compute_volume_per_tvl`. Never `Infinity`. |
| **I1317-7** | GET LEFT JOINs stamps only. Ranking, gems, and `volume_per_tvl` stay #1263. EXPLAIN must not mention `swap_events`, `pair_reserves`, `protocol_fee_events`, `liquidity_events`, `lp_holder_deltas`, or `lp_custody_checkpoints`. |
| **I1317-8** | `limit` / `window` allowlist unchanged. 60s cache holds the full response. Tests call `reset_protocol_top_pairs_cache`. |

## Invariants (P1317)

| ID | Rule |
|----|------|
| **P1317-1** | Three columns after Vol/LP: **CMM v2 LP**, **30d fee bps**, **Vol/CMM LP**. Do not remove or redefine Vol/LP. |
| **P1317-2** | CMM LP uses `formatProtocolUsd`. Fee bps uses `formatFeeBps` (no `%`). Volume uses `formatVolumePerTvl` (`2.469` → `2.47×`). |
| **P1317-3** | Missing keys, null, `≤ 0`, non-finite → em dash. Original four columns still render. |
| **P1317-4** | Tooltips: time-weighted CMM shares at current USD per share; the first balance check is a fixed anchor and a later check does not replace that history with today’s balance; bps exclude wrap/window; multiple is not full-pool Vol/LP. |
| **P1317-5** | No farm, APR, yield, or Provide CTA. |
| **P1317-6** | `overflow-x-auto` stays. New header and numeric cells are `whitespace-nowrap`. Do not drop columns. |
| **P1317-7** | Test ids `protocol-top-pair-cmm-lp-*`, `protocol-top-pair-fee-bps-*`, `protocol-top-pair-cmm-ratio-*`. |
| **P1317-8** | Verify: `make verify-issue-1317`. Keep `verify-issue-1263` and `verify-issue-1269` green. |

## Do / don’t

- **Do** hook both refreshes next to `refresh_pair_volumes_30d`.
- **Do** insert the custodian LCD anchor once, then apply deltas and a reconciliation difference on later reads.
- **Don’t** move `observed_at` or `shares` on a 24h LCD refresh.
- **Don’t** call `liquidity_event_exists` to dedupe LP-holder deltas.
- **Don’t** scan raw events or LCD on `GET /api/v1/protocol/top-pairs`.
- **Don’t** LCD every LP holder. Custodian queries only.
- **Don’t** sum `liquidity_events.provider` as the CMM balance.
- **Don’t** re-rank the top 5 or change full-pool Vol/LP.
