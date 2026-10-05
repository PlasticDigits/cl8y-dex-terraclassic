# Route-tax rollout script — agent review and verification

Issue: [#1381](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1381).
Implementation acceptance: [AC1–AC10](../issue-1381/README.md).
Operator procedure: [runbook](../../runbooks/route-local-tax-1381.md).
Rollout invariants: [RTROLL1381-1–RTROLL1381-10](../../../skills/AGENTS_ROUTE_TAX_ROLLOUT.md).

The rollout upgrades existing addresses and rotates creation templates. Review
source automation separately from any production migration: a passing script test
does not establish that ALPHA, KENA, the router or the live application was upgraded.

## Release targets and read-only baseline

The October 4, 2026 production probe identified the following baseline. Re-probe
before preparing a new manifest; these observations are not migration receipts.

| Component | Observed code / CW2 | Reviewed rollout action |
|---|---|---|
| Factory | 11629 / 1.10.0 | Store/migrate reviewed 1.11.0. The PR changes CW2 metadata only relative to repository factory code; live 1.10.0 has a different hash from the prior repository artifact, so a same-version migration is rejected. |
| Router | 11640 / 1.1.0 | Store/migrate funding-version-1 router 1.2.0. |
| Pair template | 11676 / 1.18.0 | Store reviewed 1.19.0, rotate factory template and migrate inventoried behind-version pairs. |
| Launcher | 11632 / 1.0.0 | Retain compatible wasm, rotate `token_code_id`, preserve AutoLP/unrelated configuration. |
| CMM | 11667 / 0.2.2 | Existing dispatcher meets the prerequisite; use actual `migrate_owned_contract` for its tokens, with no unnecessary CMM upgrade. |

Affected-token instances and their rates, admins, bindings and code hashes come
from the new inventory, including zero-rate instances. All four core artifacts
must be rebuilt/reviewed for the current revision: a stale factory 1.10.0 artifact
cannot satisfy the 1.11.0 target. Compare full SHA-256 records, not a CW2 label or
hash prefix. Original route funding invariants and exact pair credit remain intact.

## How an agent completes the checklist

Use an isolated worktree at the PR revision. Read the
[shell entrypoint](../../../scripts/upgrade-1381-route-tax.sh) and
[engine](../../../scripts/lib/route_tax_rollout.py), run the entrypoint with
`--help`, then run and retest the rollout tests documented below. Planning/probing
must not require access to local signing keys. Inspect a generated plan against
independent live queries when endpoints are available.

```bash
bash scripts/upgrade-1381-route-tax.sh --help
python3 scripts/qa/test-route-tax-rollout.py
# Repeat after changes to prove the fixes did not break resume/configuration:
python3 scripts/qa/test-route-tax-rollout.py
# Repository wrapper for the rollout checks:
make verify-route-tax-rollout
```

The behavioral rollout suite uses a fake transport to exercise planner/executor
decisions, transaction reconciliation and failure handling; it broadcasts nothing
and does not prove real SDK signing or production migration. Rehearse those paths
on a dedicated LocalTerra fleet where available, then use read-only production
inventory to check official deployed state. Live signing still requires an
explicitly authorized operator invocation.

The actual SDK rehearsal is
[`localterra-route-tax-rollout.py`](../../../scripts/qa/localterra-route-tax-rollout.py).
It rejects every other chain, creates its own factory/router/tokens/pair and
synthetic multisig keys, exercises the production transport, checks pause/restore
and code pins, and creates a token through the rotated launcher. Existing shared
application contracts are not migrated and shared chain volumes are not reset.

For the full CMM path, populate the public `ROLL1381_*` variables below from
reviewed artifacts and pre-v1 LocalTerra code stores. Each `*_SHA256` is the
expected hash in the reviewed release record; do not substitute an unreviewed
artifact hash merely to bypass a mismatch. Select a fresh external state directory
for every rehearsal and a public evidence JSON path:

```bash
python3 scripts/qa/localterra-route-tax-rollout.py \
  --artifacts "$ROLL1381_ARTIFACTS" \
  --state-dir "$ROLL1381_STATE_DIR" \
  --legacy-factory-code "$ROLL1381_FACTORY_CODE" \
  --legacy-factory-sha256 "$ROLL1381_FACTORY_SHA256" \
  --legacy-pair-code "$ROLL1381_PAIR_CODE" \
  --legacy-pair-sha256 "$ROLL1381_PAIR_SHA256" \
  --legacy-router-code "$ROLL1381_ROUTER_CODE" \
  --legacy-router-sha256 "$ROLL1381_ROUTER_SHA256" \
  --legacy-token-code "$ROLL1381_TOKEN_CODE" \
  --legacy-token-sha256 "$ROLL1381_TOKEN_SHA256" \
  --cmm-wasm "$ROLL1381_CMM_WASM" \
  --cmm-sha256 "$ROLL1381_CMM_SHA256" \
  --legacy-cmm-wasm "$ROLL1381_LEGACY_CMM_WASM" \
  --legacy-cmm-sha256 "$ROLL1381_LEGACY_CMM_SHA256" \
  --evidence-file "$ROLL1381_EVIDENCE_FILE"
```

Use `--container <localterra-container>` if the default container name differs.
The reviewed CMM artifacts are treasury **0.2.1** and **0.2.2**; all four CMM
arguments must be supplied together. Omitting all four explicitly skips the CMM
dispatcher check while the remaining rehearsal still runs. The real SDK test
proves local stores, migrations, governance signatures and new-token creation;
it still does not establish Columbus-5 deployment or the original issue's routed
swap accounting acceptance.

For each row record **PASS**, **FAIL**, or **SKIP**, the revision, UTC time,
command and evidence path. Run checks that are feasible before declaring a skip.
Agents may skip a check they cannot complete and still mark overall verification
**PASS with explicit skips** if every executed check passes. State the concrete
limitation and leave skipped deployment criteria unchecked/unverified. Never
relabel an executed failing assertion as a skip or PASS. This skip policy permits
source review to finish without available signing/device custody; it does not
disable transaction identity, version, tax or custody controls during execution.

## Review checklist

| Check | Agent instructions and required evidence |
|---|---|
| Read-only default | Invoke help and read-only planning/probing with test fixtures. Assert no broadcast, signing, keyring prompt, contract instantiate or production write occurs. |
| Inventory completeness | Reconcile factory pair pagination, community-tax assets, launcher pin/code instances and explicit ALPHA. Include official zero-rate KENA; reject wrong factory/router/admin and incomplete/erroring discovery. |
| Reviewed artifact identity | Compare local SHA-256 with manifest and stored-chain checksums. Confirm chain/address/version/admin bindings; reject manifest edits and same-version different-hash migrations. |
| Prerequisite upgrades | Exercise behind/current/incompatible router, token, factory, pair and launcher versions. Confirm factory/pair references point to reviewed templates and unchanged versions are not forcibly migrated. |
| Contract administrators | Inspect direct EOA, multisig, treasury/governance and CMM payloads. Confirm `cl8ydeploy` stores code, two valid multisig components authorize governance actions and CMM capability is proven before `migrate_owned_contract`. |
| Preserved config | Compare before/after launcher AutoLP and unrelated config, whitelist entries, token rates/sinks/exemptions and registered pairs. Confirm token migrations precede pin refresh and affected instances retain their addresses. |
| Pause lifecycle | Confirm original pauses are journaled, affected markets pause before migration, errors leave containment, and explicit finalization restores only pauses changed by this run. |
| Resume/reconciliation | Interrupt after submission and after inclusion. Resume using the same plan/journal; confirm pending transaction lookup, no duplicate store/migrate and failure handling. |
| Secret handling | Inspect process invocation, files, logs and fixtures for passphrase/token persistence. Verify one in-memory passphrase prompt and public-only emitted deployment environment snippets. |
| Application handoff | Inspect trusted trading route-tax IDs/hashes, tax-free reviewed identities and Create/Manage/launcher pins in emitted output. Apply them only after live hash checks; restart indexer, invalidate route caches and deploy matching frontend. |
| Protocol regression | Run `make verify-issue-1381` and record all completed/skipped groups separately from rollout automation coverage. Follow original acceptance checklist for actual funding/quote/atomicity behavior. |
| Production read-only | Probe official router and every affected token: code/hash/admin, funding version 1, configured router, tax previews, whitelist/pair pins and original settings. Save public responses; no receipt means no migration claim. |
| Production execution | With explicitly authorized custody run the reviewed plan and reconcile inclusion/code IDs. Finalize after matching application deployment and read-only checks, then immediately conduct a bounded synthetic zero-intermediate-balance swap; paused pairs cannot execute it. Record quote, taxes/sinks, pair credit, refunds, output and gas. Re-pause affected markets on failure. |

## Evidence boundaries and remaining operator work

No production broadcast is required to review this PR. For an explicitly requested
production execution, keyring, 2-of-3 or wallet work unavailable to the agent is
an explicit skip and a cl8y-pm inbox card, with
one action and at most five numbered taps under repository AGENTS.md. Include the
DEX issue URL. Keep production AC9 open until actual migration and execution
receipts exist; a read-only successful preview is not execution evidence.

Review rollback against
[SEC-H05](../../runbooks/wasm-admin-migration.md#rollback-and-limitations-sec-h05).
A downgrade is not guaranteed by the stored prior code ID: check migration version
guards and storage compatibility before choosing recovery. Keep the fleet paused
and use a reviewed forward repair when a downgrade is unsupported.

Recorded results and public synthetic receipts: [RESULTS.md](RESULTS.md).
