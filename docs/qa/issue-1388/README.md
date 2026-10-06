# Issue #1388: agent review and verification

Issue: [#1388](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1388).
[Invariants and third-party agent procedure](../../../skills/AGENTS_ROUTER_TRUST_LIFECYCLE.md),
[recovery runbook](../../runbooks/router-trust-1388.md),
[funding regression checklist](../issue-1381/README.md),
[rollout checklist](../issue-1381-rollout/README.md), [results](RESULTS.md).

## Reproduce

Use an isolated worktree at the PR revision, Rust, Node 24 and a dedicated test
Postgres database. Run `make verify-issue-1388`, then `make verify-issue-1381`.
The issue verifier runs focused contract, discovery and behavioral rollout checks twice,
checks formatting and documentation, and reports available/unavailable groups.
The broader verifier exercises funding/frontend and actual synthetic LocalTerra
fixtures; rebuild current optimized wasm before chain execution.

For each row record PASS/FAIL/SKIP, revision, UTC time, exact command and evidence
path. Complete feasible provisioning and runnable checks first. **Agents may skip
items they cannot complete and mark overall verification PASS with explicit
skips**, provided every executed check passes. Record the concrete limitation and
leave its issue criterion unverified. Executed failures remain FAIL and must be
fixed or reported; do not relabel them SKIP. Unavailable workflow holds do not
prevent source implementation, commit or PR. This policy never waives on-chain
identity/accounting controls or establishes production deployment.

## Acceptance and agent actions

| Issue criterion | Agent procedure and evidence |
|---|---|
| AC1: in-place rollout | Run rollout tests/retest. Confirm wrong/new-address bindings fail before broadcasts, including unpaired and zero-rate inventories. Run a synthetic existing-address migration with reviewed prior wasm, refresh pair pins and execute funded first/middle/final routes. Compare balances/config/dust and migration receipts. |
| AC2: rotation recovery | Run contract trust tests. Cover freshly instantiated, foreign-adopted and same-crate migrated tokens. Validate live-admin or bound-factory-governance replacement/revocation, including an opaque contract token admin without a dispatcher, exact candidate code/hash/factory/capability and immutable/factory-governed custody, active/malformed candidate rejection and atomic failure. Assert stale router calls fail and overlapping exemptions survive. Rehearse compiled-wasm rotation on LocalTerra. |
| AC3: real code identity | Run rollout artifact tests and discovery unit/integration tests twice. A shared CW2 label cannot override mismatched full hashes. Exercise same-instance ordinary→funded→unsupported code migration within cache TTL, identity lookup failures and mid-discovery migration. Re-run existing catalog/cw2 spoof rejection tests; origin alone is insufficient. |
| AC4: settlement identity | Run contract replay tests and full funding regressions. Cover canonical one-time emission, duplicate slot under the same context, distinct slots, identical transfers in distinct same-height routes, pre-existing router dust and later-hop rollback of consumption/counters/taxes/balances. Inspect bounded state and overflow rejection. |
| Production recovery | Independently query every live affected contract's code ID/checksum, router binding, rates, exemptions and capability. Actual authorized migration/rotation and bounded route receipts are required for deployment acceptance. Source tests and read-only probes cannot check this row. |

## Compiled-wasm rehearsal

```bash
# Build current release wasm first in smartcontracts/.
python3 scripts/qa/localterra-route-tax-1381.py \
  --legacy-artifacts /path/to/reviewed-earlier-router-token-wasm \
  --code-cache /tmp/issue1388-codes.json --evidence-file /tmp/issue1388-funding.json
python3 scripts/qa/localterra-router-trust-1388.py \
  --fixture /tmp/issue1388-funding.json --evidence-file /tmp/issue1388-trust.json
```

Run `python3 scripts/qa/test-localterra-route-tax-1388.py` twice to verify bounded
CheckTx sequence-rejection recovery and that accepted/unknown transactions are
never resent. Each funding fleet generates its own synthetic owner and tax sink;
fixture evidence carries public test-key names for follow-on helpers. Older
fixtures retain test1/test2 compatibility. Use `--container <isolated-container>`
when shared LocalTerra is busy; never reset another agent's volumes.

Use a fresh funding fleet before each trust rehearsal: rotation intentionally
changes that synthetic token binding. The helper rejects every chain except
LocalTerra and checks unauthorized/hash/factory failures, successful replacement,
stale-route rollback, revocation and recovery with actual compiled code. It never
migrates shared application contracts or resets volumes. Set
`VERIFY_ISSUE_1388_CHAIN_COMMAND` to the available rehearsal command to incorporate
its result; otherwise record `VERIFY_ISSUE_1388_SKIP_CHAIN_REASON`. Other group
skips use `VERIFY_ISSUE_1388_SKIP_CONTRACTS_REASON`, `..._INDEXER_REASON`,
`..._ROLLOUT_REASON` and `..._LIVE_REASON`. Production execution is separate.

## Useful read-only probe

Run `python3 scripts/qa/probe-route-tax-1381.py --help` and supply the actual
reviewed code-ID/SHA-256 records with `--reviewed-code CODE_ID=SHA256` plus
`--expect-reviewed-artifacts --expect-funded`, the inventoried router, every token,
a listed pair and synthetic trader. Enumerate the full inventory; a single-token
probe is not fleet acceptance. This sends no signing material and broadcasts
nothing. Keep public receipts synthetic and free of credentials.
