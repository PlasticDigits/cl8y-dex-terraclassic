# Receive sizing and reverse budget repair (#1376)

Implementation: [PR #1378](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1378).
Issue: [#1376](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1376).
Invariants: [E1376-1–E1376-9](../../frontend.md#solver-receive-sizing).
API/search scope: [exact-output sizing](../../route-solver.md#exact-output-sizing).
Third-party agent protocol: [AGENTS_SWAP_EXACT_OUTPUT.md](../../../skills/AGENTS_SWAP_EXACT_OUTPUT.md).
Implementation evidence: [verification.md](verification.md).
Latest deployment, live failure and explicit custody waiver: [live-verification.md](live-verification.md).

Exact-output is receive sizing for an offer-in transaction. The supported solver scope is pool-only, up to four hops and the first five feasible fresh candidates among 32 structural paths. Hybrid book optimization remains offer-in; receive sizing does not claim an unrestricted optimum. Existing mapper-only inverse quotes and router/invoice reverse consumers remain supported.

## Agent verification procedure

1. Record `git rev-parse HEAD`, install Node 24 dependencies, and ensure Postgres using `make setup-indexer-postgres` if needed. Never run host Cargo through a root bind-mount container. Run `make verify-issue-1376` and retain stdout plus exit status. It runs realistic contract fixtures, solver unit/Postgres API tests, frontend service/RTL tests twice, browser lifecycle, lint/types and docs checks.
2. Run `make verify-issue-1256`, `make verify-issue-713`, `make verify-issue-678`, `make verify-issue-512`, `make verify-issue-1280`, `make test-contracts` and `make lint-contracts`. Correct failures and rerun the affected checks. Do not use a skip variable to conceal an executed assertion failure.
3. Review the checklist below against actual test assertions and diff. Inspect gross offer, operations, trader/tax context, pre-tax minimums and stale-response invalidation. Do not check off a broad acceptance criterion solely because one mocked happy path passed.
4. Attempt `make has-localterra` then provision `make setup-cloud-localterra` when feasible. Use test-funded simulated-wallet transactions for native-enter, direct and multi-hop routes. Capture exact gross send and operations/minimums; verify target coverage or protected failure after state changes. Set `VERIFY_ISSUE_1376_CHAIN_COMMAND` to the actual chain test command when running the verifier. For dedicated Vite ports, merge their explicit local origins with `scripts/lib/indexer-cors-playwright.sh` and restart the test indexer; do not change production CORS. Run funded tests serially and use a unique Playwright `--output` directory.
5. After authorized deployment, perform the versioned inventory/probes and fresh-browser replay below. Record bundle/indexer revisions, pair code/cw2, token resolution, block and snapshots. Production migration/signing is separate from a test-fixture pass.

For fresh read-only browser replay, run `node scripts/qa/browser-probe-issue-1376.cjs --expect-fixed --matrix`; set `PROBE_1376_BASE_URL` for a test deployment. The matrix uses 16 new disconnected contexts across `/` and `/swap`, Compact/Full and cold/warm reload, records public bundle paths and selected quote fields, and clicks no wallet or transaction control. Inspect matching forward operations as part of AC1; browser probe output alone is insufficient for funded execution.

An agent may report **PASS with explicit skips** when it cannot complete a check and all completed checks pass. Record **SKIP**, the precise missing capability/prerequisite and remaining scope for every unavailable item. A skipped item is never a passed item; an executed failing assertion remains **FAIL**. This task explicitly allows unavailable holds/gates to be skipped for overall implementation review, without changing runtime safety or claiming production completion. Do not assign verification work to the human.

The verifier accepts `VERIFY_ISSUE_1376_SKIP_CONTRACTS_REASON`, `..._INDEXER_REASON`, `..._FRONTEND_REASON`, `..._BROWSER_REASON`, `..._LIVE_REASON` and `..._CHAIN_REASON`. Each must explain actual unavailability. `VERIFY_ISSUE_1376_LIVE=1` expects migrated live pair queries to pass; without deployment it records a separate live SKIP.

## Review and acceptance checklist

Reusable reviewer checklist; execution statuses and completed issue criteria are in [verification.md](verification.md).

- [ ] **AC1:** Open both issue URLs on `/` and `/swap` in fresh disconnected/connected browsers, cold/warm cache and compact/full display. Receive remains exactly `10`/`100`; Pay is positive, solver carries `amount_out`, and matching forward operations cover the target. Record public token addresses and actual versions.
- [ ] **AC2:** Run `api_route_exact_output` and frontend solver tests. Compare competing paths; an invalid/infeasible first candidate does not hide an alternate. Indexer-only output intent remains output. Check published candidate scope and explicit unsupported hybrid/taxed-middle policy.
- [ ] **AC3:** Run workspace `issue1376` contract tests. Replay exact reserve/180-bps fixtures, old 32/35 calls, both orientations and 6/18, 18/6, 18/18. Assert sufficient offer and insufficient offer-minus-one where minimality is claimed. Empty/overflow/infeasible targets and hybrid exhaustion terminate safely.
- [ ] **AC4:** Run mapper/tax/router/invoice tests plus funded chain execution. Inspect fee-tier context, buy net/pre-tax bases, sell extra-debit once, intermediate tax support/rejection, wrap fees and unwrap fee then burn-tax/cap/rate limits. Test paused/unavailable mapper and fee boundaries.
- [ ] **AC5:** Run disconnected RTL/browser errors: loading, 404 no-route, deterministic 422 and transient network/429/503/504; executable Pay clears, Retry is visible and bounded, and recovery preserves Receive. No auto-wallet action.
- [ ] **AC6:** Run rapid `10→100→10`, clear, flip, pickers, Max/fractions, settings/wallet-context, URL back/reload/share and refetch failures. Late responses cannot restore old executable Pay or operations.
- [ ] **AC7:** Inspect submit assertions and funded test-chain messages. Send selected gross offer with the selected route and fresh forward quote; minimums remain pre-tax where required. Existing balance/LUNC, pause/freeze/blacklist, Expert and spread guards remain effective.
- [ ] **AC8:** Run verifier, regressions, lint/types, contract lint and Postgres tests; inspect docs/OpenAPI/playbook links and deployment inventory. Record exact PASS/FAIL/SKIP, including any missing live execution.
- [ ] **Abuse:** Reject non-ASCII/negative/exponent/overlong/>Uint128 targets before work. Verify conflicting modes, stale mirrors/fidelity, missing decimals, forged ops/hops, same-token/no-path, capacity/timeout and unsupported native hybrid. No cache-bucket executable reuse or browser brute-force loops.

## Pair migration inventory and read-only probes

Pair cw2 changes from **1.18.0 to 1.19.0**; storage and `MigrateMsg {}` are unchanged. The repaired query requires migrating every factory-listed existing pair; new pairs require the factory's `pair_code_id` to point at the new code. The router ABI and offer-in execute semantics remain compatible. The frontend/indexer solver can avoid the old reverse query, but that does not prove legacy direct/invoice consumers on old pair code are fixed.

Before deployment, run:

```bash
python3 scripts/qa/probe-issue-1376.py > /tmp/issue-1376-before.json
sha256sum smartcontracts/artifacts/cl8y_dex_pair.wasm
```

The public probe paginates factory pairs (bounded to 300, explicitly flags truncation), records public code IDs/cw2/assets and block context, then compares real 10/100 reverse queries with sufficient forward queries. It does not sign, read a wallet/keyring or publish admin/account identities. `--lcd`, `--factory`, `--pair` and `--target-token` select a test deployment; the defaults are the issue's public Columbus-5 contracts. Do not resolve a ticker solely from `token_info.symbol`; the deployed registry may display CL8Y while token metadata says CL8Y-cb.

For release, rebuild the final revision's optimized artifact, record SHA/cw2, check storage-compatible migration and actual admin authority internally, use existing migration tooling with reviewed **1.19.0** configuration, update factory pair code, migrate inventoried pairs in bounded batches and retain transaction receipts. Do not blindly reuse the older #1324 script's default expected version. Preserve reserves, LP supply, TWAP and order/refund state. Abort mismatched artifact/authority rather than implying a successful migration. Record inventory omissions/truncation.

After deployment, rerun:

```bash
python3 scripts/qa/probe-issue-1376.py --expect-fixed > /tmp/issue-1376-after.json
VERIFY_ISSUE_1376_LIVE=1 make verify-issue-1376
```

Compare every inventory row's new code/version, reverse/forward outputs and state. Repeat direct/router/invoice regressions and fresh browser URLs. Record current block/reserves: fixture prices are historical evidence, never a permanent quote promise. A complete production acceptance requires sufficient Pay with matching solver/forward route evidence, not merely a successful pair query. If custody/deployment is unavailable, record that scope as SKIP and complete the independently available implementation review.

Local test-only migration helper: `python3 scripts/qa/migrate-local-pairs-1376.py --factory <local-factory> --evidence-file /tmp/issue1376-local.json`. It refuses every chain except `localterra`, verifies test-account admin internally, verifies artifact version, updates factory code and checks each migrated pair code/cw2 and unchanged pool response. Resume with `--code-id <stored-id>` only when the stored checksum matches the final artifact. [Completed local migration receipts](localterra-migrations.json) are test evidence, not Columbus-5 migration evidence.

Funded execution and native guard check: `PLAYWRIGHT_WEB_PORT=3086 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-tx --workers=1 --output=/tmp/issue1376-chain-artifacts e2e/swap-receive-1376-tx.spec.ts`. A native guard annotation is proof of protection, not proof of a native broadcast.

## Optional custody-dependent rollout

The live verification task explicitly allows skip-and-close for work requiring
unavailable operator custody. Closure under that waiver preserves the failed
legacy reverse result; it does not mean pair migration or full chain acceptance
passed. See [the recorded disposition](live-verification.md). No operator action
is a prerequisite to that waived closure.

The prepared workstation command is:

```bash
bash /home/answorld/repos/cl8y-dex-verify-1376/scripts/qa/complete-issue-1376.sh --migrate
```

The agent prepared the tested artifact (SHA-256
`4ad54cd3b9a5ff3d28b80bfec21800b39fee97ca9295bb524353db7590005bba`)
in this worktree. The command checks that checksum and unchanged contract
sources, selects cw2 1.19.0, then uses the existing file-keyring / 2-of-3 tool.
`--dry-run` validates the artifact without signing; `--check` only probes.
The existing migration tool checks pair count/admin and preserved observation
bytes. A partial rollout can resume with `UPGRADE1324_SKIP_STORE=1` and
`UPGRADE1324_PAIR_CODE_ID`; the wrapper checks the stored Wasm checksum before
reuse. Keep custody input and transaction working files local. If a guard
fails, stop and let an agent investigate; never bypass it or automatically
downgrade pair code. Agents own post-migration probe review.
