# Fee-payer cohorts (#1211)

Apply to fee actors, registry history, cohort materialization or `/protocol/fees`.
Issue: [#1211](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1211).
QA and live limitations: [checklist](../docs/qa/issue-1211/README.md).
Fee source/provenance home: [L1213](AGENTS_INDEXER_FEE_LEDGER_HOME.md).

| Invariant | Required behavior |
|---|---|
| FC1211-1 | `protocol_fee_attributions` privately stores one payer/cohort per economic fee id. Insert only; replay cannot rewrite actor, cohort or stamped fee USD. Missing/legacy attribution rolls up as `unknown`. Fee deletion cascades attribution on explicit reorg recovery. |
| FC1211-2 | MM requires an explicit registered tier 0. Unregistered default tier 0, self-register 1–9 and tier 255 are retail. Never classify from live `traders`, behavioral studies or a wallet allowlist. |
| FC1211-3 | An exact-provider-height H−1 registry snapshot plus pinned registry receipts ordered by H/transaction/action determines membership at each fee. Later same-transaction deregistration must not affect an earlier fee. Missing/malformed/history-unavailable is unknown; never substitute current-height state. A malformed pinned transition invalidates that actor; an unidentifiable wallet is an unknown barrier for subsequent fees until an explicit valid transition. |
| FC1211-4 | Block attribution has at most four historical reads within one two-second absolute observation deadline, beginning with its first read. Propagate the original deadline into an owned LCD view before constructing read futures; retain post-response deadline rejection. Existing chain identity, endpoint, permit and historical-height guards remain enforced. No GET or quote request triggers this work. |
| FC1211-5 | Resolve swaps from the authenticated parsed pair/index and exact emitted sender/assets/amount tuple. A trusted pinned router may supply `trader`; other senders cannot override their own identity. Old router receipts without an attributable trader are unknown. Book takes follow the linked parent swap taker, never the maker. Placement actor requires its stored pair/order identity; ambiguous matches are unknown. |
| FC1211-6 | Wrap/window actors come from the same pinned action and ordinal stream as economic ingestion. Invoice/factory actors must match the production parser's emitter/source/ordinal; custom `wasm-limit_order_fill` events do not enter that stream. Missing payer attributes, malformed addresses, duplicate actor keys and operational contract identities are unjoined. Do not infer a payer from manager, receiver or transaction funder. |
| FC1211-7 | Public `by_cohort` is three static aggregate rows (`retail`, `mm`, `unknown`); no actors, registry events, tier dump, wallet roster, or campaign fields. Shares use stamped priced USD only. Unpriced activity increments event count, with null USD/share; configured idle is `"0"`. |
| FC1211-8 | Scheduled global/source/token/cohort publication uses one repeatable-read transaction. GET reads one read-only repeatable-read snapshot and caches the aggregate response for 60 seconds. GET never scans fee events or joins actors. |
| FC1211-9 | Reorg recovery deletes registry events/snapshots at and after the rewind and cascades fee attribution. Historical snapshots below the rewind stay valid. No automatic rewrites of historical cohorts from reconciled/live tiers. |
| FC1211-10 | Local synthetic receipts prove implementation behavior, not deployed payer emission or completed live evidence. Current pair/factory/token invoice contracts omit some required payer fields; keep those paths unknown and report that limitation. Runtime never migrates: the separate migrator must prepare this additive schema before deploying the new indexer. |

Run `make verify-issue-1211`. Build commands may exceed a minute; each actual test
process is bounded to 59 seconds. Keep missing live/LocalTerra evidence explicit.
