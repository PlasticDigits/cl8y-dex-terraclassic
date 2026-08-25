# Agent playbook: wrap/unwrap protocol-fee ingest (GitLab #613)

Audience: third-party agents changing wrap-mapper fee ingest, `GET /protocol/fees` wrap/unwrap rows, or `/protocol` Source labels for Wrap / Unwrap.

**Issue:** [GitLab **#613**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/613)  
**Parent census:** [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) (**PFee-1–PFee-12**, [#586](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586))  
**Split fees (query-time bps, not ingest):** [`AGENTS_WRAP_MAPPER_SPLIT_FEES.md`](./AGENTS_WRAP_MAPPER_SPLIT_FEES.md) (**W12–W15**)  
**Invariants table:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Protocol fees #586 / #613**)  
**Ops:** [`docs/runbooks/overview-global-stats-brin.md`](../docs/runbooks/overview-global-stats-brin.md) § Protocol fees  
**Sibling (out of scope here):** [#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614) UST1 window mint/redeem fees — [`AGENTS_INDEXER_UST1_WINDOW_FEES.md`](./AGENTS_INDEXER_UST1_WINDOW_FEES.md) (**I614**)

## Problem class

`/protocol` advertised Wrap / Unwrap as fee sources (**PFee-3**) but production `GET /api/v1/protocol/fees` stayed at `event_count: 0` after #586 ingest landed. #586 assumed `action=wrap|unwrap` + `fee_amount`. Live ustr-cmm wrap-mapper emits **`action=notify_deposit`** (wrap) / **`action=unwrap`** with amount key **`fee`**. Last-value parse on a flattened wrap+swap wasm stream also dropped the mapper segment when the last `action` was `swap` / `instant_withdraw`.

## Captured attrs (do not guess)

Locked from [ustr-cmm wrap-mapper](https://gitlab.com/PlasticDigits2/ustr-cmm) `contracts/wrap-mapper/src/contract.rs` (`execute_notify_deposit`, `execute_receive_cw20`) and fixtures in [`protocol_fees.rs`](../indexer/src/indexer/protocol_fees.rs). Columbus-5 mapper: `terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2` ([`REGISTRY.md`](../deployments/mainnet-ust1-wrap/REGISTRY.md)).

| Path | Emitter | `action` | Amount key | Token key | Source key |
|------|---------|----------|------------|-----------|------------|
| Direct wrap / wrap_deposit + router | mapper (after treasury `wrap_deposit`) | `notify_deposit` | `fee` | `denom` (`uusd` / `uluna`) | `wrap` |
| CW20 `send` unwrap / router `unwrap_output` | mapper | `unwrap` | `fee` | `denom` | `unwrap` |

Treasury `wrap_deposit` / `instant_withdraw` are **not** fees (`amount` is gross / withdraw). InstantWithdraw burn tax / `tax_amount` are **not** protocol fees (**W8–W11**, **M590-4**). Legacy `action=wrap` + `fee_amount` is still accepted as a #586 alias.

## Invariants (I613)

| ID | Rule |
|----|------|
| **I613-1** | Map captured `notify_deposit` → source `wrap`, `unwrap` → source `unwrap`. Retail labels stay **Wrap** / **Unwrap**. Do not add cUSTC/cLUNC source rows — token mix stays in the Tokens table. |
| **I613-2** | Scan **each** `action` segment. Scope by reserved `_contract_address` only (#285). Flattened wrap_deposit + `notify_deposit` + `swap` still persists wrap **and** `swap_amm` once each. Forged `contract_address` (no underscore) is ignored. |
| **I613-3** | Fail closed on missing `fee` / `fee_amount` or token identity. **Never** infer `gross × fee_wrap_bps`. Zero / non-numeric / negative → drop. |
| **I613-4** | Pin is exact `terra1` bech32 (`WRAP_MAPPER_ADDRESS`). Spoof `notify_deposit` / `wrap` on another contract is ignored. Unconfigured pin **omits** wrap/unwrap from `by_source` (not fake `$0`). |
| **I613-5** | Same P522-Q catalog: `uusd`/cUSTC → USTC; `uluna`/cLUNC → LUNC. Never vFDUSD. Never `$1` UST1. Stamp `fee_usd` at ingest (#568). Unpriced → store event, `fee_usd` NULL; window with only unpriced wrap → overview `null` (`—`). |
| **I613-6** | GET `/overview` and `/protocol/fees` stay O(1) rollup / 60s cache. Do not `SUM protocol_fee_events` on the request path. |
| **I613-7** | `/protocol` Source shows Wrap / Unwrap when `event_count > 0`; idle `$0` still hidden (**PFee-3**). |
| **I613-8** | This skill + invariants + `make verify-issue-613`. Keep `make verify-issue-586`. |

## Do / don’t

- **Do** lock attr names from a captured tx or ustr-cmm source before changing the parser.
- **Do** treat wrap and unwrap as additional treasury inflows next to AMM/book — not a substitute for L7.
- **Don’t** count `wrap_deposit` / `instant_withdraw` / `tax_amount` / `hook_fee_amount` / spread / gas.
- **Don’t** bind-mount `indexer/` into root Docker for cargo (`make test-indexer-target-ownership`).
- **Don’t** treat `wrap_mapper_configured: true` with `event_count: 0` as done.
- **Don’t** implement UST1 window mint/redeem here (#614).

## Key files

| Area | Path |
|------|------|
| Parser + pin | [`indexer/src/indexer/protocol_fees.rs`](../indexer/src/indexer/protocol_fees.rs) `parse_wrap_fees` |
| Ingest hook | [`indexer/src/indexer/parser.rs`](../indexer/src/indexer/parser.rs) `process_wrap_fee` |
| Rollup / omit | [`indexer/src/db/queries/protocol_fees.rs`](../indexer/src/db/queries/protocol_fees.rs) |
| Pin env | [`indexer/src/config.rs`](../indexer/src/config.rs) `WRAP_MAPPER_ADDRESS` |
| UI | [`ProtocolFeeStats.tsx`](../frontend-dapp/src/components/protocol/ProtocolFeeStats.tsx) |
| Tests | [`protocol_fees.rs`](../indexer/src/indexer/protocol_fees.rs) unit, [`indexer_protocol_fees.rs`](../indexer/tests/indexer_protocol_fees.rs), [`ProtocolPage.test.tsx`](../frontend-dapp/src/pages/ProtocolPage.test.tsx) |

## Regression

```bash
make setup-indexer-postgres
make verify-issue-613
make verify-issue-586
make verify-issue-616
```

Optional LocalTerra capture (not required when fixtures match ustr-cmm): `make setup-cloud-localterra`, wrap `uusd` + `uluna`, unwrap both, one wrap+router combo; indexer must persist `wrap` / `unwrap`.

## Related

- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) — **PFee-6** points here
- [`AGENTS_INDEXER_UST1_WINDOW_FEES.md`](./AGENTS_INDEXER_UST1_WINDOW_FEES.md) — window mint/redeem sibling ([#614](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/614))
- [`AGENTS_POST_MERGE_OPS_616.md`](./AGENTS_POST_MERGE_OPS_616.md) — post-merge stack child ([#616](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/616))
- [`AGENTS_WRAP_MAPPER_SPLIT_FEES.md`](./AGENTS_WRAP_MAPPER_SPLIT_FEES.md) — ingest ≠ query-time bps
- [`AGENTS_WRAP_UNWRAP_BURN_TAX.md`](./AGENTS_WRAP_UNWRAP_BURN_TAX.md) — W8–W11
- [`AGENTS_POST_MERGE_OPS_590.md`](./AGENTS_POST_MERGE_OPS_590.md) — Coolify pin (**M590-2**), unwrap `fee` only (**M590-4**)
- [`AGENTS_INDEXER_CANDLE_USD_MARK.md`](./AGENTS_INDEXER_CANDLE_USD_MARK.md) — stamp USD at ingest (#568)
- [`NATIVE_TOKEN_WRAPPING.md`](../NATIVE_TOKEN_WRAPPING.md)
