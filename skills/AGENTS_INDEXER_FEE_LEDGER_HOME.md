# Agent playbook: fee-ledger home (#1213)

Use when an agent is asked to add pair-creation, SKU/settings invoices, or retail/MM cohort splits to `GET /api/v1/protocol/fees`, or to “open a fee ticket on marketing.”

This is the **design/home map**. It does **not** add a `FeeSource`, migration CHECK, API field, or UI label.

**Issue:** [Forgejo **#1213**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1213)  
**Live census (seven sources today):** [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) (**PFee-1–PFee-13**, [#586](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/586))  
**Wrap ingest pattern:** [`AGENTS_INDEXER_WRAP_FEE_INGEST.md`](./AGENTS_INDEXER_WRAP_FEE_INGEST.md) (**I613**)  
**Window ingest pattern:** [`AGENTS_INDEXER_UST1_WINDOW_FEES.md`](./AGENTS_INDEXER_UST1_WINDOW_FEES.md) (**I614**)  
**Catalog ≠ fees:** [`AGENTS_INDEXER_COMMUNITY_TOKENS.md`](./AGENTS_INDEXER_COMMUNITY_TOKENS.md) (**I594**)  
**Invariants table:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Fee-ledger home #1213**)  
**Verify:** `make verify-issue-1213`

## Owned children (do not reopen ingest here)

| Slice | Implement | Closed marketing tracker |
|-------|-----------|--------------------------|
| Pair-creation treasury uluna on `GET /api/v1/protocol/fees` (no instantiate-gas double-count) | [#1209](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1209) | [cl8y-marketing#1](https://git.cl8y.com/PlasticDigits/cl8y-marketing/issues/1) |
| Community SKU unlock + settings-batch invoices on the same API | [#1210](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1210) | [cl8y-marketing#4](https://git.cl8y.com/PlasticDigits/cl8y-marketing/issues/4) |
| Actor-joined priced fees with fee-discount-registry cohort split (retail / MM / unjoined) | [#1211](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1211) | [cl8y-marketing#5](https://git.cl8y.com/PlasticDigits/cl8y-marketing/issues/5) |

Stack order: **#1209 → #1210 → #1211**. Copy wrap/window pin + explicit-amount + uniqueness from **I613** / **I614**. Do not file a fourth feat that copies those titles.

Related, **not this epic:**

- [#1204](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1204) — OpenAPI/curl pack (no ingest).
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
- [`AGENTS_INDEXER_WRAP_FEE_INGEST.md`](./AGENTS_INDEXER_WRAP_FEE_INGEST.md) — pattern to copy on children
- [`AGENTS_INDEXER_UST1_WINDOW_FEES.md`](./AGENTS_INDEXER_UST1_WINDOW_FEES.md) — pattern to copy on children
- [`AGENTS_INDEXER_COMMUNITY_TOKENS.md`](./AGENTS_INDEXER_COMMUNITY_TOKENS.md) — catalog only
- [`docs/audits/factory-treasury-bank-send.md`](../docs/audits/factory-treasury-bank-send.md) — item 11 until #1209
- [`docs/qa/issue-1213/README.md`](../docs/qa/issue-1213/README.md)
