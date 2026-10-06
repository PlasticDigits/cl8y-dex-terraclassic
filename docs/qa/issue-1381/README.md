# Issue #1381 — implementation review and verification

Issue: [#1381](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1381).
Protocol/invariants: [RT1381-1–RT1381-10](../../../skills/AGENTS_ROUTE_LOCAL_TAX.md).
Deployment/version compatibility: [runbook](../../runbooks/route-local-tax-1381.md).
Production script review: [rollout checklist](../issue-1381-rollout/README.md).
Routing: [solver](../../route-solver.md); product flows: [frontend](../../frontend.md).

The required outcome is actual route-funded execution, including zero intermediate
wallet balances. Excluding unsupported old versions is rollout containment only.
Source verification and test-chain execution cannot establish Columbus-5 deployment.

## Reproducible agent checks

Work in an isolated git worktree with Node 24, Rust, PostgreSQL and LocalTerra.
Use the existing [Postgres playbook](../../../skills/AGENTS_LOCAL_POSTGRES_DEV.md)
and a dedicated test database; do not reset shared chain/DB volumes. Build current
optimized wasm (`cd smartcontracts && bash scripts/optimize.sh`) and provide the
reviewed `cw20_mintable.wasm` dependency in `smartcontracts/artifacts`.

```bash
# Include real migration and native paths when reviewed fixtures are available:
VERIFY_ISSUE_1381_LEGACY_ARTIFACTS=/path/to/pre-v1-artifacts \
VERIFY_ISSUE_1381_WRAP_STACK_ENV=/path/to/localterra/frontend-dapp/.env.local \
make verify-issue-1381
# Optional actual pre-v1 → v1 migration fixture, using reviewed old artifacts:
python3 scripts/qa/localterra-route-tax-1381.py \
  --legacy-artifacts /path/to/reviewed-pre-v1-artifacts \
  --evidence-file /tmp/dex-1381-migration.json
```

The verifier tests and retests contract funding/accounting, trading discovery and
Postgres APIs, actual frontend hooks/pages/invoice budgets, then checks format,
frontend lint/types/build, documentation and isolated chain execution, including atomic invoice/native wrap/unwrap flows. The chain
script creates its own factory/router/pools/accounts. `--isolated-owner` funds a fresh
synthetic setup signer for parallel QA; the verifier enables it. Only rejected
CheckTx sequence mismatches have bounded retries, never accepted transactions
or ambiguous transport errors. It signs only on LocalTerra
with the test keyring and rejects every other chain. Its output includes raw gas,
code hashes, public receipts and synthetic token accounting, never private keys.

Agents may skip an item they cannot complete and still mark overall verification
**PASS with explicit skips**. First attempt feasible provisioning/execution, then
record the unavailable check, concrete reason, revision and UTC time. Leave that
criterion unchecked/unverified. Never relabel an executed failing assertion as
PASS; fix it or report FAIL. Skipped live migration does not satisfy AC9 and does
not justify closing the issue as deployed. This policy overrides unavailable
workflow holds, while keeping all protocol/accounting/security controls intact.

`VERIFY_ISSUE_1381_LEGACY_ARTIFACTS` enables actual pre-v1→v1 migration.
`VERIFY_ISSUE_1381_WRAP_STACK_ENV` selects an existing LocalTerra public wrap stack;
the helper reads only treasury/mapper/wrapped-token addresses, checks immutable
wrapped-token code, and never reads wallet mnemonics. Without it native checks
remain explicitly skipped; invoice checks still execute. Run the additional
[`localterra-route-tax-flows-1381.py`](../../../scripts/qa/localterra-route-tax-flows-1381.py)
on the base fixture to repeat invoice/native success and atomic failures.

Reason variables: `VERIFY_ISSUE_1381_SKIP_CONTRACTS_REASON`,
`VERIFY_ISSUE_1381_SKIP_INDEXER_REASON`, `VERIFY_ISSUE_1381_SKIP_FRONTEND_REASON`,
`VERIFY_ISSUE_1381_SKIP_STATIC_REASON`, `VERIFY_ISSUE_1381_SKIP_CHAIN_REASON`,
`VERIFY_ISSUE_1381_SKIP_LIVE_REASON`. Use `VERIFY_ISSUE_1381_LIVE_COMMAND` only for
an actually available authorized coordinated migration and execution check;
read-only capability probes alone cannot pass that group.

## Acceptance checklist for the reviewing agent

| Criterion | Agent action and required evidence |
|---|---|
| AC1 | Run contract `issue1381` multitests and LocalTerra script twice. Confirm CL8Y→ALPHA→UST1→USTR and multiple-tax routes succeed from fresh synthetic accounts with zero intermediate/output holdings. |
| AC2 | Reconcile every buy/sell boundary and sink/supply changes. Inspect token `apply_transfer`: routed tax uses router debit, never trader subtraction. Confirm authenticated trader controls remain. |
| AC3 | Match gross `G`, maximal economic `S`, tax, pair credit/hook, pool fees, net output and payer residue. Check the 1,010,000→1,000,000+10,000 example, integer extremes, explicit hybrid/greedy and forward/reverse parity. |
| AC4 | Compare before/after input/intermediate wallets, allowances and router dust. Pre-existing holdings must remain unused; only the authorized initial budget can leave the wallet. Verify owned residue, swap mutex and settlement context. |
| AC5 | Run `api_route_tax_funding1381`: supported intermediate absent catalog is eligible; adverse alternative ranked by executable net; legacy/spoofed/unknown versions fail closed. |
| AC6 | Run GET/POST/manual hybrid, exact-output, progress/cache and client fallback tests. Inspect all candidate code/hash/config/exemption cache identities. Version-1 router output is already net, never taxed twice. |
| AC7 | Run actual Swap/Trade/hook/invoice budget tests plus client execution preflight. Check gross Max/typed/percent/receive inputs, missing balance, Create feature disabled and failed refetch. Include available native/wrap browser checks; list unavailable devices separately. |
| AC8 | Run full token/router/contract regressions: zero rates, separate-token exemptions, cooldown, spoofing, pause/freeze/blacklist, legacy direct-pair/SendFrom allowance, LP/limit/refund/AutoLP behavior. |
| AC9 | Run actual legacy-artifact LocalTerra migration fixtures and read-only deployed capability/code/hash/preview probes. For Columbus-5 follow coordinated router/token store+migrate+whitelist/pin refresh and cache rollout, then synthetic bounded execution. Do not check production acceptance without actual receipts. |
| AC10 | Exercise failed first/middle/final execution, sink/min-return rollback and stale/spoofed context. Compare balances/sinks/supply/route state. Check funding vs ordinary error copy, no intermediate-reserve advice and no automatic resend. |

## Results

Results are recorded in [RESULTS.md](RESULTS.md), with completed checks, limitations
and evidence boundaries. Follow the commands above to independently reproduce and
replace a skip with evidence. Do not use the investigation's old #1285/#1267 green
checks as proof of funded protocol correctness.
