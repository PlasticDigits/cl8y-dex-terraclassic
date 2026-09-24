# Agent playbook: fee-ledger home (#1213)

Use when an agent is asked to add pair-creation, SKU/settings invoices, or retail/MM cohort splits to `GET /api/v1/protocol/fees`, or to “open a fee ticket on marketing.”

This is the **design/home map**. It does **not** add a `FeeSource`, migration CHECK, API field, or UI label.

**Issue:** [Forgejo **#1213**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1213)  
**Live census (seven sources today):** [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) (**PFee-1–PFee-14**, [#586](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586))  
**Multihop AMM uniqueness:** [`AGENTS_INDEXER_PROTOCOL_FEE_HOPS.md`](./AGENTS_INDEXER_PROTOCOL_FEE_HOPS.md) (**F1269** / [#1269](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269)) — children inherit this **widened** key, not `UNIQUE (tx_hash, source, ordinal)`  
**Wrap ingest pattern:** [`AGENTS_INDEXER_WRAP_FEE_INGEST.md`](./AGENTS_INDEXER_WRAP_FEE_INGEST.md) (**I613**)  
**Window ingest pattern:** [`AGENTS_INDEXER_UST1_WINDOW_FEES.md`](./AGENTS_INDEXER_UST1_WINDOW_FEES.md) (**I614**)  
**Catalog ≠ fees:** [`AGENTS_INDEXER_COMMUNITY_TOKENS.md`](./AGENTS_INDEXER_COMMUNITY_TOKENS.md) (**I594**)  
**Invariants table:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Fee-ledger home #1213**)  
**Marketing pointers (not a fee service):** [`PlasticDigits/cl8y-marketing` `strategy/fee-ledger-home.md`](https://git.cl8y.com/PlasticDigits/cl8y-marketing/src/branch/main/strategy/fee-ledger-home.md) and [`research/snapshots/README.md`](https://git.cl8y.com/PlasticDigits/cl8y-marketing/src/branch/main/research/snapshots/README.md) — companion [PR #7](https://git.cl8y.com/PlasticDigits/cl8y-marketing/pulls/7) until merged.  
**Verify:** `make verify-issue-1213`

**Invoice child status:** [#1210](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1210) is still open; SKU/settings invoice ingest is not implemented on `main`. See [QA #1210](../docs/qa/issue-1210/README.md) and the **L1210-1–L1210-8** implementation contract below. The #1213 verifier proves the home map only.

## Owned children (do not reopen ingest here)

| Slice | Implement | Closed marketing tracker |
|-------|-----------|--------------------------|
| Pair-creation treasury uluna on `GET /api/v1/protocol/fees` (no instantiate-gas double-count) | [#1209](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1209) | [cl8y-marketing#1](https://git.cl8y.com/PlasticDigits/cl8y-marketing/issues/1) |
| Community SKU unlock + settings-batch invoices on the same API | [#1210](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1210) | [cl8y-marketing#4](https://git.cl8y.com/PlasticDigits/cl8y-marketing/issues/4) |
| Actor-joined priced fees with fee-discount-registry cohort split (retail / MM / unjoined) | [#1211](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1211) | [cl8y-marketing#5](https://git.cl8y.com/PlasticDigits/cl8y-marketing/issues/5) |

Stack order: **#1209 → #1210 → #1211**. Copy wrap/window pin + explicit-amount + uniqueness from **I613** / **I614**. Inherit the **widened** `protocol_fee_events` unique key from **#1269** / **F1269** (pair-scoped `swap_amm`; NULL-pair partial for non-pair sources). Do not land a copy of `UNIQUE (tx_hash, source, ordinal)`. Do not file a fourth feat that copies those titles.

Related, **not this epic:**

- [#1204](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1204) — OpenAPI/curl pack (no ingest). Playbook [`AGENTS_INDEXER_HTTP_PACK.md`](./AGENTS_INDEXER_HTTP_PACK.md).
- [#1202](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1202) — source-to-fee client events (privacy hold; not fee truth). #1211 depends on that hold remaining; do not expand #1202.
- [#594](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/594) — community-token **catalog**.
- [#597](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/597) — MM subscription invoices (not SKU/settings).

## Invariants (L1213-1–L1213-8)

| ID | Rule |
|----|------|
| **L1213-1** | Indexer `protocol_fee_events` is the governed ledger. Marketing stores pointers and labeled snapshots only. Do not implement a second public `/protocol/fees` or wasm parser in `PlasticDigits/cl8y-marketing`. |
| **L1213-2** | Pair-creation ingest is **#1209**. Do not count instantiate gas, overpay refunds, or `reply_instantiate_pair` as treasury. Audit item 11 stays discovery-only until #1209 ships. |
| **L1213-3** | SKU unlock + settings-batch invoices are **#1210**. `community_token_events` answers “which token unlocked which SKU,” not treasury USD. One paid invoice → one fee row. Do not mix into `pair_creation` or `swap_amm`. |
| **L1213-4** | Cohort split is **#1211** (aggregate retail / MM / unjoined as-of height). Unregistered `traders.tier_id = 0` is **not** Tier 0 MM. No wallet lists, no person-to-wallet map, no `campaign_id` on fee rows. `traders.total_fees_paid` must not drive `/protocol` headlines. |
| **L1213-5** | Marketing #1 / #4 / #5 stay **closed** trackers. Do not reopen them for implement/ready. Historical “closed tracker” mentions may remain if they also name the dex replacement. |
| **L1213-6** | GET `/protocol/fees` stays O(1) rollup / 60s cache. Unconfigured factory/launcher pin **omits** the source (not fake `$0`). Idle configured → `"0"`. Unpriced → `null`. Pin emitters; forged `contract_address` without underscore is ignored ([#285](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/285)). |
| **L1213-7** | Do not expand #1204 or #1202 from this epic. No social/X/paid-media work. No fourth ingest copy. Do not churn labels on #1209 / #1210 / #1211. |
| **L1213-8** | This skill + `docs/indexer-invariants.md` + factory audit item 11 pointer + `make verify-issue-1213`. Child tickets keep their own fail-closed ACs. |

## #1210 invoice-ingest contract (L1210-1–L1210-8)

These are implementation invariants for [#1210](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1210), not claims about the current code. The current `FeeSource` set and protocol-fee API do not include community invoices.

| ID | Rule |
|----|------|
| **L1210-1** | `protocol_fee_events` is the only treasury-fee ledger. Community catalog `community_token_events` describes token/SKU lifecycle; it does not prove a CMM inflow. Use separate `sku_unlock` and `settings_fee` sources, not `pair_creation` or `swap_amm`. |
| **L1210-2** | Create-time SKUs use the pinned launcher's `create_token.sku_count × 50_000_000` UST1 once; zero SKUs produce no row. Later `EnableFeature` uses one positive token `invoice` attr. `UpdateSettings` uses one positive `invoice` for the whole batch, never a per-field multiple. |
| **L1210-3** | One paid invoice produces one fee row. Do not count `create_token_ready`; do not double-count launcher and token `enable_feature` segments in one tx. |
| **L1210-4** | Scan flattened wasm per action and accept only reserved `_contract_address`. Require the exact pinned `COMMUNITY_TOKEN_LAUNCHER` for create-time invoices. Token events require a catalogued community token with trustworthy launcher/CMM provenance; `GetLauncherOrigin` alone is insufficient while [#1229](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1229) is open. |
| **L1210-5** | Amount must parse, be positive, and represent the actual CMM UST1 invoice. Fail closed on missing, zero, malformed, reverted, or no-op invoices. Pin UST1 by CW20 contract identity; never trust `symbol=` or assume `$1`. |
| **L1210-6** | Exclude migrate-adopt, tax skim, AutoLP, gas, burn tax, and #595 pay-with-any-token swap/wrap legs. Keep factory `pair_creation` (#1209), MM subscription (#597), and cohort attribution (#1211) separate. |
| **L1210-7** | Add sources additively to fee rollups, `/protocol/fees`, `/protocol/fees/daily`, DeFiLlama, and UI labels. GET remains O(1) / 60s cached: unconfigured pins omit sources, configured idle is `"0"`, and unpriced activity is `null`. |
| **L1210-8** | Inherit the widened #1269 uniqueness: pair-scoped `(tx_hash, source, pair_id, ordinal)` for non-null pair IDs and the partial `(tx_hash, source, ordinal)` index for `pair_id IS NULL`. Do not restore an unconditional three-column UNIQUE. Add parser/API/UI regressions and paid-transaction evidence. |

### Systems to change when #1210 is implemented

- Parser and source model: [`indexer/src/indexer/protocol_fees.rs`](../indexer/src/indexer/protocol_fees.rs), [`indexer/src/indexer/parser.rs`](../indexer/src/indexer/parser.rs), [`indexer/src/indexer/community_tokens.rs`](../indexer/src/indexer/community_tokens.rs).
- Persistence and API: [`indexer/src/db/queries/protocol_fees.rs`](../indexer/src/db/queries/protocol_fees.rs), [`indexer/src/api/protocol_fees.rs`](../indexer/src/api/protocol_fees.rs), [`indexer/src/api/protocol_fee_series.rs`](../indexer/src/api/protocol_fee_series.rs), [`indexer/src/api/defillama.rs`](../indexer/src/api/defillama.rs), and protocol-fee migrations.
- Emitter and display: [`smartcontracts/contracts/community-tax-token/src/invoice.rs`](../smartcontracts/contracts/community-tax-token/src/invoice.rs), [`smartcontracts/contracts/community-token-launcher/src/contract.rs`](../smartcontracts/contracts/community-token-launcher/src/contract.rs), [`frontend-dapp/src/types/index.ts`](../frontend-dapp/src/types/index.ts), and [`frontend-dapp/src/components/protocol/ProtocolFeeStats.tsx`](../frontend-dapp/src/components/protocol/ProtocolFeeStats.tsx).
- Keep catalog #594 and catalog provenance #1229 in scope for emitter trust; keep widened uniqueness from [#1269](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269). Add `make verify-issue-1210` with the implementation; it does not exist yet.

## Rules of thumb

1. **Pick the child, not this epic.** Pair creation → #1209. Invoices → #1210. Cohort → #1211.
2. **Indexer wins after ship.** Offline LCD notebooks stay labeled **provisional** until the matching ingest lands, then retire the stopgap in marketing snapshots.
3. **Do not mix sources.** Pair-creation uluna, SKU/settings UST1, swap/wrap/window, and client analytics are four different truths.
4. **Do not relabel children** from this ticket.

## Verification

```bash
make verify-issue-1213
```

Docs-only. Child ingest tickets keep `make verify-issue-586` / `make verify-issue-614` plus their own bundles.

## Cross-links

- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) — seven-source census (**PFee**)
- [`AGENTS_INDEXER_PROTOCOL_FEE_HOPS.md`](./AGENTS_INDEXER_PROTOCOL_FEE_HOPS.md) — `swap_amm` pair-scoped unique (**F1269**, [#1269](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269)); inherit on children
- [`AGENTS_INDEXER_WRAP_FEE_INGEST.md`](./AGENTS_INDEXER_WRAP_FEE_INGEST.md) — pattern to copy on children
- [`AGENTS_INDEXER_UST1_WINDOW_FEES.md`](./AGENTS_INDEXER_UST1_WINDOW_FEES.md) — pattern to copy on children
- [`AGENTS_INDEXER_COMMUNITY_TOKENS.md`](./AGENTS_INDEXER_COMMUNITY_TOKENS.md) — catalog only
- [`docs/audits/factory-treasury-bank-send.md`](../docs/audits/factory-treasury-bank-send.md) — item 11 until #1209
- [`docs/qa/issue-1213/README.md`](../docs/qa/issue-1213/README.md)
- [`docs/qa/issue-1210/README.md`](../docs/qa/issue-1210/README.md) — current verification; #1210 remains open
- Marketing `strategy/fee-ledger-home.md` — pointer table only; do **not** implement wasm parsers there
