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
| **I1317-2** | `cmm_lp_usd = tw_shares / total_supply × pair_liquidity_usd`. Share-seconds over trailing 30 UTC days. Provide credits `receiver`. Withdraw debits `sender`. User LP transfers move both sides. Ignore LP mint/burn and sends whose counterparty is the pair. |
| **I1317-3** | No checkpoint → NULL (not zero). Checkpoint inside the window: flat shares until `observed_at`, then deltas. Negative running balance clamps to 0. LCD checkpoint is off the GET path. |
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
| **P1317-4** | Tooltips: time-weighted CMM shares at current USD per share; flat prefix before the first balance check; bps exclude wrap/window; multiple is not full-pool Vol/LP. |
| **P1317-5** | No farm, APR, yield, or Provide CTA. |
| **P1317-6** | `overflow-x-auto` stays. New header and numeric cells are `whitespace-nowrap`. Do not drop columns. |
| **P1317-7** | Test ids `protocol-top-pair-cmm-lp-*`, `protocol-top-pair-fee-bps-*`, `protocol-top-pair-cmm-ratio-*`. |
| **P1317-8** | Verify: `make verify-issue-1317`. Keep `verify-issue-1263` and `verify-issue-1269` green. |

## Do / don’t

- **Do** hook both refreshes next to `refresh_pair_volumes_30d`.
- **Do** seed custody from an off-request LCD checkpoint, then apply deltas.
- **Don’t** scan raw events or LCD on `GET /api/v1/protocol/top-pairs`.
- **Don’t** sum `liquidity_events.provider` as the CMM balance.
- **Don’t** re-rank the top 5 or change full-pool Vol/LP.
