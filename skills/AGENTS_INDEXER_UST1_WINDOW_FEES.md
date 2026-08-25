# Agent playbook: UST1 window mint/redeem protocol-fee ingest (GitLab #614)

Audience: third-party agents changing ust1-window fee ingest, `GET /protocol/fees` mint/redeem rows, `/protocol` Source labels **UST1 mint** / **UST1 redeem**, or Coolify indexer `UST1_WINDOW_ADDRESS`.

**Issue:** [GitLab **#614**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614)  
**Parent census:** [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) (**PFee-1–PFee-13**, [#586](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586))  
**Execute UI (out of scope):** [`AGENTS_UST1_WINDOW_UI.md`](./AGENTS_UST1_WINDOW_UI.md) (`/ust1`, [#506](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/506))  
**Sibling wrap ingest:** [`AGENTS_INDEXER_WRAP_FEE_INGEST.md`](./AGENTS_INDEXER_WRAP_FEE_INGEST.md) (**I613**, [#613](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/613))  
**Invariants table:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Protocol fees #586 / #613 / #614**)  
**Ops:** [`docs/runbooks/overview-global-stats-brin.md`](../docs/runbooks/overview-global-stats-brin.md) § Protocol fees · [`docs/runbooks/ust1-window-ui.md`](../docs/runbooks/ust1-window-ui.md)  
**Registry:** [`deployments/mainnet-ust1-wrap/REGISTRY.md`](../deployments/mainnet-ust1-wrap/REGISTRY.md)  
**Verify:** `make verify-issue-614`

Do **not** fold window fees into `wrap` / `unwrap` or `swap_amm`. Do **not** infer `ust1_out × fee_total_bps`. Vite `VITE_UST1_WINDOW_ADDRESS` does **not** feed the indexer.

## Problem class

`/protocol` already counted AMM / book / wrap treasury inflows. The UST1 oracle window (vFDUSD↔UST1 CW20 `Send`) charges a separate CMM fee on a different contract. Production 11566 wasm emitted `fee_*_bps` / `vfdusd_to_treasury` without `fee_amount`, so ingest must **fail closed**. Same address migrated to **11618** ([ust1-window#33](https://gitlab.com/PlasticDigits/ust1-window/-/issues/33)) now emits `fee_amount` + `fee_asset` (UST1). Coolify must pin the **indexer** env.

## Captured attrs (do not guess)

| Wasm | `action` | Amount key | Token key | Source | Retail label |
|------|----------|------------|-----------|--------|--------------|
| 11618 (live) | `deposit` | **`fee_amount`** | `fee_asset` (UST1 CW20) | `ust1_mint` | **UST1 mint** |
| 11618 (live) | `withdraw` | **`fee_amount`** | `fee_asset` (UST1 CW20) | `ust1_redeem` | **UST1 redeem** |
| 11566 (legacy, same address) | `deposit` / `withdraw` | *none* (`fee_*_bps`, `ust1_out`, `vfdusd_to_treasury` only) | — | drop | — |

Pin: `terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2` (do not instantiate a second window). Token allowlist also accepts `fee_denom` / `native_denom` / `denom` / `ust1_token`. Never `symbol=` (A1). Never price with CEX vFDUSD / `$1` UST1 / `2.5×` USTR — hub UST1 only (**PFee-7** / **H1–H10**).

`min_vfdusd_out`, oracle spread, Venus redeem, Classic burn tax, gas, rolling 24h unused capacity, and pause rejects are **not** fees.

## Invariants (I614)

| ID | Rule |
|----|------|
| **I614-1** | Pin is `UST1_WINDOW_ADDRESS` (same terra1 rules as wrap). Do **not** reuse `WRAP_MAPPER_ADDRESS`. Empty / garbage → omit mint/redeem (not fake idle `$0`). |
| **I614-2** | `deposit` → `ust1_mint`, `withdraw` → `ust1_redeem`. Retail labels **UST1 mint** / **UST1 redeem**. Never show `deposit` / `withdraw` / `effective_swap`. |
| **I614-3** | Require explicit positive `fee_amount` + token identity on the window `_contract_address` segment. **Never** infer `ust1_out × fee_total_bps` / `vfdusd_to_treasury × fee_cmm_protocol_bps`. 11566 bps-only attrs fail closed. |
| **I614-4** | Flattened CW20 `send` + hook scopes by reserved `_contract_address` only (#285). Forged `contract_address` (no underscore) and spoof window-shaped wasm from a non-pinned contract are ignored. |
| **I614-5** | Price with hub UST1 (**PFee-7**). Never vFDUSD/FDUSD / `$1` UST1. Stamp `fee_usd` at ingest (#568). Unpriced → store event, `fee_usd` NULL; activity + all unpriced → overview `null` (`—`). |
| **I614-6** | GET `/overview` and `/protocol/fees` stay O(1) rollup / 60s cache. Do not `SUM protocol_fee_events`. `window=` stays `24h` \| `7d` \| `30d` (400 else) — not “ust1-window”. |
| **I614-7** | `/protocol` Source shows mint/redeem when `ust1_window_configured` and `event_count > 0`. Idle `$0` hidden. Missing flag (old indexer) → hide rows, do not invent `$0` (**PFee-10**). CEX / hub cards still do **not** claim to be the window rate (**P550-11**). |
| **I614-8** | This skill + **PFee-13** + `make verify-issue-614`. Keep `make verify-issue-586`. Live Coolify pin + captured deposit/withdraw increment `event_count` (observed 2026-08-25 on `indexer.dex.cl8y.com`). Do not bind-mount `indexer/` into root Docker for cargo. |

## Do / don’t

- **Do** lock attr names from a captured columbus-5 / LocalTerra tx or ust1-window crate before changing the parser.
- **Do** keep wrap and window pins independent.
- **Don’t** count `ust1_out` / `vfdusd_out` / `vfdusd_to_treasury` / `min_vfdusd_out` as the fee.
- **Don’t** treat Vite bake as ingest. Coolify indexer must set `UST1_WINDOW_ADDRESS`.
- **Don’t** implement wrap-mapper ingest here (#613).
- **Don’t** change on-chain `fee_bps` or overload `/mint`.

## Key files

| Area | Path |
|------|------|
| Parser + pin | [`indexer/src/indexer/protocol_fees.rs`](../indexer/src/indexer/protocol_fees.rs) `parse_ust1_window_address` / `parse_ust1_window_fees` |
| Ingest hook | [`indexer/src/indexer/parser.rs`](../indexer/src/indexer/parser.rs) |
| Rollup / omit | [`indexer/src/db/queries/protocol_fees.rs`](../indexer/src/db/queries/protocol_fees.rs) |
| Pin env | [`indexer/src/config.rs`](../indexer/src/config.rs) `UST1_WINDOW_ADDRESS` |
| CHECK | [`indexer/migrations/20260824120000_ust1_window_protocol_fees.sql`](../indexer/migrations/20260824120000_ust1_window_protocol_fees.sql) |
| UI | [`ProtocolFeeStats.tsx`](../frontend-dapp/src/components/protocol/ProtocolFeeStats.tsx) |
| Tests | [`protocol_fees.rs`](../indexer/src/indexer/protocol_fees.rs) unit, [`indexer_protocol_fees.rs`](../indexer/tests/indexer_protocol_fees.rs), [`ProtocolPage.test.tsx`](../frontend-dapp/src/pages/ProtocolPage.test.tsx) |
| Coolify | [`deployments/mainnet-ust1-wrap/coolify.env.example`](../deployments/mainnet-ust1-wrap/coolify.env.example) |

## Regression

```bash
make setup-indexer-postgres
make verify-issue-614
make verify-issue-586
# optional live leftover (pin + mint/redeem event_count):
VERIFY614_REQUIRE_LIVE=1 make verify-issue-614
```

## Related

- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) — **PFee-13** points here
- [`AGENTS_UST1_WINDOW_UI.md`](./AGENTS_UST1_WINDOW_UI.md) — `/ust1` execute; CEX/hub cards are **not** the window rate
- [`AGENTS_INDEXER_WRAP_FEE_INGEST.md`](./AGENTS_INDEXER_WRAP_FEE_INGEST.md) — wrap/unwrap sibling
- [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) — hub UST1 USD
- [`AGENTS_INDEXER_CANDLE_USD_MARK.md`](./AGENTS_INDEXER_CANDLE_USD_MARK.md) — stamp USD at ingest (#568)
- [`AGENTS_POST_MERGE_OPS_616.md`](./AGENTS_POST_MERGE_OPS_616.md) — stack child; do **not** reopen #614 for ops/QA
