# Oracle integrity and hub containment (#1391)

Issue: [#1391](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1391).
Implementation: [PR #1412](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1412); results and revision in [verification.md](verification.md).
Invariants: [O1391](../../indexer-invariants.md#oracle-integrity-and-hub-containment-1391).
Agent playbook: [AGENTS_ORACLE_SAFETY.md](../../../skills/AGENTS_ORACLE_SAFETY.md).
API: [external oracle](../../runbooks/indexer-external-oracle.md).
LocalTerra attempts and reproduction: [chain-probe.md](chain-probe.md).

## Price policy

`cex-quorum-v1` accepts finite prices in **1e-12..1e6 USD** from distinct
allowlisted KuCoin, MEXC and CoinGecko source identities. At least **two** agree
within a **1.1 max/min ratio**; ambiguous equally supported clusters fail closed.
Only accepted inliers contribute to the aggregate. Zero, negatives, NaN,
infinity, overflow and decimal underflow are rejected before storage; conversion
never substitutes zero. Failure immediately clears current certified state.
`oracle_state` stores price, observation timestamp, accepted source names and
quality atomically; legacy average rows alone do not certify a current quote.
The current lookup rejects future observations and expires after **300 seconds**.
New average history stores accepted source identities. Legacy samples remain explicitly unverified; historical repairs require certified averages within 300 seconds before the historical event, never an arbitrary old price or a symbol-spoofed wrap. Historical qualification does not mean current availability.

`as_of`/`fetched_at` retain the original local poll-start time, captured before HTTP
and database work. This conservative bound prevents a slow peer or write from
renewing an earlier source response. It certifies poll recency and source agreement,
not authenticated exchange market age or fair value. A provider serving an old
value with HTTP 200 and two agreeing replayed feeds can still pass; exchange
timestamp authentication is outside this policy. Independent bounded ticker and
hub schedules follow [#1464 recovery](../../runbooks/indexer-provider-recovery.md);
quorum and expiry policies remain unchanged.

Ticker APIs publish `quality`, `as_of`, `age_seconds`, `max_age_seconds`,
`source_count`, `quorum`, `source_policy`, and `accepted_sources`. Only `fresh`
exposes a current price; `degraded`, `stale` and `unavailable` expose null. Path
`vfdusd` still means CEX **FDUSD**, never the Terra CW20 or a fabricated $1 peg.
Venus remains a separate exact pinned BSC vToken redeem observation, also age
bounded. Downstream wraps retain independent USTC and LUNC sources.

`live-reserve-spot-v1` requires exact configured CW20 identities, humanized
reserves and **$100 minimum independently priced leg depth** (configurable through
existing `HUB_USD_TVL_FLOOR`). Candidate self-valued total TVL cannot qualify it.
UST1 uses the actual canonical UST1/cUSTC pool; no duplicate canonical pair attack
is assumed. USTR candidates retain deterministic ties.
Spot expiry follows the ticker even when a source-pair foreign key is removed; loss of that key cannot turn a spot into a longer-lived external quote. Idle candle classification also requires exact canonical wrap contracts and consistent native/CW20 flags.
Each scheduled pass publishes the latest fresh, positive reserve spot immediately.
There is no fixed-reference band, no confirmation delay and no drift ratchet: large
or fast genuine price movement is delivered as the new spot and never suppresses a
mark. Missing, stale, nonpositive or below-floor-depth evidence still fails closed,
so an outage, an empty pool or a missing anchor cannot fabricate a quote. Economic
marks preserve the oldest reserve/anchor observation and require current health for
their exact USTC, LUNC, UST1 or USTR dependency; refresh cannot renew an old input. Cached overview live prices are requalified on each response. DeFiLlama daily prices remain explicitly historical advisory snapshots with original observation timestamps.
Responses disclose policy, quality, as-of, TTL and independent depth. **All hub marks have `oracle_grade=false`.**

These marks are live reserve spots; they do not establish independent fair value.
Reserve manipulation visible only at the next observation, a manipulated bootstrap
and coordinated sustained manipulation remain possible. Spot prices are unsuitable
for collateral, settlement, automated spending authorization or treasury targets.
No inspected settlement math uses these USD marks, and this change makes no
collateral-theft claim. Financial consumers keep their separate fresh-quorum and
movement checks (below); the quote path never blocks on price magnitude.

## Financial actions

All repository indexer-fed mint/buyback/LP actions use the shared
[Python validator](../../../scripts/lib/oracle_safety.py). Exact ticker/quote identity,
fresh source quorum and timestamps are mandatory. Rechecks immediately before
transaction preparation stop stale/degraded responses and **>1% movement** relative
to the plan. Venus target calculations also verify pinned source, contract and age.
Manual price overrides are supported only under `DRY_RUN=1`; they do not fabricate
fresh source observations for live actions. Existing UST1=$1 and USTR seed ratios in
LP sizing remain explicit operator strategy constants, never certified hub marks.
These scripts need the new indexer API before live use; old payloads fail closed.

## Agent verification checklist

Record SHA, UTC time, commands, exit statuses and evidence scope in a receipt.
Use an isolated Postgres DB in ignored `indexer/.env`; never reset shared chain or
production databases. Run available checks first and fix/retest executed failures.

- [ ] **AC1:** Run `make verify-issue-1391`; inspect parser/outlier/quorum tests,
  legacy and stale replay rejection, outage and recovered-quorum Postgres fixtures.
- [ ] **AC2:** Run canonical-pool Postgres and fixed-clock hub tests. Confirm a
  reserve shift publishes the moved live spot (never a suppression), independently
  priced depth loss fails closed and propagated UST1/dependent marks degrade when
  the anchor is missing. Inspect hub, overview and DeFiLlama quality/provenance fields.
- [ ] **AC3:** Run financial consumer mock tests included in the verifier. For each
  actual prepare function, reject expired/future/degraded/no-quorum/spoofed input;
  verify the mocked terrad log stays empty, including deterioration after planning.
- [ ] **AC4:** Run canonical identity, decimals and USD/adapter suites included in
  the verifier; review generated OpenAPI and the stated remaining spot risk.
- [ ] **Chain attack:** Probe `make has-localterra`; provision
  `make setup-cloud-localterra` if absent. On isolated funded canonical pools,
  execute a reserve manipulation and LP withdrawal, retain actual block/timestamp
  provenance, and verify the moved spot is published live while depth loss fails
  closed. Never alter production.
- [ ] **Deployment:** Deploy migrations/indexer and capture read-only timestamped
  CEX/hub/API/adapter responses; confirm moved reserve spots are served and verify
  financial dry runs reject degraded data. Do not execute production spending as QA.

Agents may report **overall PASS with explicit skips** when unavailable checks are
recorded with exact attempted prerequisites/reasons and all executed checks pass.
Leave skipped acceptance boxes unchecked. This permission overrides unavailable
holds/gates for implementation review; it does not turn a skip into an executed
pass, certify deployment, or hide an assertion failure. Verifier skip variables
`VERIFY_ISSUE_1391_SKIP_INDEXER_REASON` and `VERIFY_ISSUE_1391_SKIP_SCRIPTS_REASON`
require a reason. Supply isolated chain checks with
`VERIFY_ISSUE_1391_CHAIN_COMMAND`; otherwise record `VERIFY_ISSUE_1391_SKIP_CHAIN_REASON`.
Deployment scope uses `VERIFY_ISSUE_1391_SKIP_LIVE_REASON`. No human checklist is
required: verification belongs to agents.
