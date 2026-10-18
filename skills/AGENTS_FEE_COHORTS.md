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
| FC1211-6 | Wrap/window actors come from the same pinned action and ordinal stream as economic ingestion. Captured mapper `notify_deposit` authenticates the treasury callback and names the fee-paying `depositor`; admit that field only for this pinned action and reject duplicate depositor keys. An unwrap `recipient` never establishes its payer; missing sender stays unknown. Invoice/factory actors must match the production parser's emitter/source/ordinal; custom `wasm-limit_order_fill` events do not enter that stream. Missing payer attributes, malformed addresses, duplicate actor keys and operational contract identities are unjoined. Do not infer a payer from manager, receiver or transaction funder. |
| FC1211-7 | Public `by_cohort` is three static aggregate rows (`retail`, `mm`, `unknown`); no actors, registry events, tier dump, wallet roster, or campaign fields. Shares use stamped priced USD only. Unpriced activity increments event count, with null USD/share; configured idle is `"0"`. Cohort USD and percent strings use the existing #557 `bd_plain_string` helper, including dust values; never emit scientific notation. |
| FC1211-8 | Scheduled global/source/token/cohort publication uses one repeatable-read transaction. GET reads one read-only repeatable-read snapshot and caches the aggregate response for 60 seconds. GET never scans fee events or joins actors. |
| FC1211-9 | Reorg recovery deletes registry events/snapshots at and after the rewind and cascades fee attribution. Historical snapshots below the rewind stay valid. No automatic rewrites of historical cohorts from reconciled/live tiers. |
| FC1211-10 | Local synthetic receipts prove implementation behavior, not deployed payer emission or completed live evidence. Current pair/factory/token invoice contracts omit some required payer fields; keep those paths unknown and report that limitation. Runtime never migrates: the separate migrator prepares this additive schema. The compatibility server may serve 67 first with cohort features explicitly pending; it never migrates itself. |

| FC1211-11 | Compatibility startup alone accepts reviewed markers 67 (`20261016120000`) and 68 (`20261018110000`) with every runtime privilege check unchanged. Maintenance/QA stay strict; future compiled migrations remove this older-schema exception. Cache one marker per block and decide rollup/GET access inside their RR snapshot. On 67, never query cohort tables; omit `by_cohort` with `cohort_status=pending_schema`. On 68, publish cohorts only after an actual atomic joint refresh stamps the existing `indexer_state` publication key and global/source/token/cohort clocks agree. Bootstrap, missing or mixed generations remain `pending_publication`, never three fabricated zeros. Preserve migration bytes and rollback compatibility; never stop the serving API for this transition. |

Run `make verify-issue-1211`. Build commands may exceed a minute; each actual test
process is bounded to 59 seconds. Keep missing live/LocalTerra evidence explicit.

Standalone rollout follows the [deployment guide](../docs/qa/issue-1211/deployment.md):
`cohort-migration-1211.py plan/build` pins exact merged main and builds without a
DB credential; only explicit inspect/stop/apply phases can advance the owned job.
Keep complete SQLx history, effective migrator DSN options, private snapshot,
new-schema runtime grants/denials, current run-ID receipts and credential cleanup.
Never infer deployed migration from the [owned-local rehearsal](../docs/qa/issue-1211/local-migration-rehearsal.json)
or initialize source cohorts as idle when historical published fees exist.

The [compatibility transition checklist](../docs/qa/issue-1211/compatibility.md)
records owned-local67→68 serving/ingestion verification separately from any
production migration or traffic switch.
