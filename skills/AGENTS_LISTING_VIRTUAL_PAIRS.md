# Agent playbook: listing-only native markets (#1437)

Apply when changing CG/GT projections, mapper principal ingestion, or opaque
listing ids. [Issue #1437](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1437),
[contract and ids](../docs/CG_CMC_COMPLIANCE.md#listing-only-native-markets-1437),
[invariants](../docs/indexer-invariants.md#listing-only-native-markets-1437),
[agent checklist/skip policy](../docs/qa/issue-1437/README.md),
[verification receipt](../docs/qa/issue-1437/verification.md).

| ID | Required behavior |
|---|---|
| LV1437-1 | Only CG and GT consume the five translated markets. Never insert opaque ids into factory pairs, swaps, liquidity, reserves, candles, TVL, or trader accounting. CMC's ticker cache stays factory-only. |
| LV1437-2 | Exact configured cLUNC/cUSTC/USDT pins and native denoms identify legs. Indexed asset symbols supply CG tickers. Gems and listing-excluded legs/sources remain excluded. Missing assets, mapper, backing, or source omit the affected market. |
| LV1437-3 | Peg principal is `mint_amount` / `withdraw_amount`, never mapper fees. Trust reserved `_contract_address` only at the configured mapper, strict bounded raw integers, a successful real indexed tx sender/hash/block, and validated historical custody at height-attested `Config.treasury` (same source/predecessor address). Never assume the mapper holds the native bank funds. Uncommitted future blocks remain invisible. |
| LV1437-4 | Crosses select the greatest stamped USD liquidity exact-leg pool. Translate human 1:1 peg amounts and postevent reserves, preserve USDT decimals, and invert source orientation once. No CEX/advisory USD price becomes a trade price. |
| LV1437-5 | Each eligible indexed source event becomes one swap per applicable market; stable source ids and replay uniqueness prevent duplicate projection. Multiple genuine source actions within one transaction retain distinct prints. Factory feeds remain unchanged. GT event indexes are unique per block/tx across the combined response. No invented trades or liquidity events. |
| LV1437-6 | Use existing CG/GT response types. Human price is target/base; GT amounts are human, CG trade/volume assignments match factory raw units. Exact BigDecimal formatting never emits scientific notation. |
| LV1437-7 | Requests issue no LCD calls for virtual markets. Historical bank reads and custody refresh run in ingestion/background on an attested LCD. A missing/wrong height response cannot supply a postevent reserve. |
| LV1437-8 | Ordinary pair/discovery paths reject the opaque namespace before SQL/LCD. CG real-ticker collisions win; GT opaque identity is independent. Pair pages slice the combined list; tickers append the fixed published order to the existing real page and cache the complete payload for 60 seconds. |
| LV1437-9 | The 5000-row GT cap counts factory rows first. Under-cap responses preserve all factory rows plus projections; over-cap remains HTTP 400. Historical trades cap at 500; orderbook has one indexed level per side and uses the existing depth cap. |
| LV1437-10 | Run available checks and retest. Unavailable items may be explicit SKIP with overall PASS WITH SKIPS; never turn an executed failure into a pass. Local checks prove implementation, not deployment or listing acceptance. |

Code: [projection](../indexer/src/api/listing_virtual_pairs.rs),
[CG](../indexer/src/api/cg.rs), [GT](../indexer/src/api/gt.rs),
[principal ingestion](../indexer/src/indexer/listing_principal.rs),
[ordinary-id guard](../indexer/src/listing_id_guard.rs).
Run `make verify-issue-1437` in an isolated worktree with disposable Postgres.
For third-party agent players, follow this playbook before extending native
listing edges; do not reuse them for settlement, route solve, or economic totals.

Related: [amount parsing](AGENTS_INDEXER_EVENT_AMOUNTS.md),
[database isolation](AGENTS_INDEXER_DATABASE_ISOLATION.md),
[chain attestation](AGENTS_CHAIN_ATTESTATION.md),
[CG/CMC truthfulness](AGENTS_INDEXER_CG_CMC_LISTING.md),
[listing exclusions](AGENTS_LISTINGS.md), [DeFiLlama](AGENTS_DEFILLAMA.md).

Production custody correction: persist the attested `custody_address`; legacy
NULL/unproven observations do not publish, and the newest unproven marker cannot
fall back to older backing. Only same-source principal fills contribute to the
peg. Wrap treasury delta is gross input; unwrap delta is net withdrawal.
Historical URLs carry height as a cache key and require matching response height.
Apply the new additive custody-source migration; never rewrite the original
#1437 migration or fund the mapper to manufacture listing backing.

For subsequent deployment and form evidence, use the
[production rollout and listing receipt](../docs/qa/issue-1437/production-listing.md).
Form submission is not crawler acceptance; recheck published edges and feature
a real factory pool before reusing the form pack.
