# #1381 verification results

Source: implementation branch `fix/1381-route-local-tax`, based on main
`3f9310652ac83818749f5d32c4e32057b08817bc`; the final implementation commit is
linked by this PR. Checks ran on 2026-10-04 UTC. Token/router are **1.2.0** with
funding protocol **1**. [Issue](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1381),
[agent checklist/skip policy](README.md),
[invariants](../../../skills/AGENTS_ROUTE_LOCAL_TAX.md),
[migration runbook](../../runbooks/route-local-tax-1381.md).

Final `make verify-issue-1381` result: **PASS=13, FAIL=0, SKIP=1** —
**PASS with explicit skips**. The only selected check skipped was Columbus-5
migration/live execution. Successful synthetic public receipts/accounting:
[route/migration evidence](localterra-evidence.json),
[atomic invoice/native evidence](localterra-flows-evidence.json).

## Executed checks

| Check | Evidence |
|---|---|
| Contract workspace, all targets | **884 passed**, zero ignored; normal workspace Clippy `-D warnings`, formatting and both schema generators passed. Includes six actual pair/router integration tests for conservation, book/hybrid/greedy, burn, context, cooldown, exemptions, dust and rollback. |
| Indexer | **513 passed**: 458 library, 12 exact-output, 29 route API, six funding API and eight DB-hybrid tests. Includes missing-catalog discovery, immutable identity, hostile capabilities, supported net ranking, per-token exemptions, exact gross cache and progress after expiry. |
| Final frontend verifier set | **19 files / 268 tests passed twice**. Actual hooks/Swap/Trade/invoice preflight, manual funded hybrid, missing/refetch balances, config/exemption failures, gross budgets, hostile route identity and gas hints. |
| Wider frontend regression | Initial 351-file run had 3,031 passes and 12 failures. Those failures were corrected and all affected files rerun successfully; subsequent 22-file/362-test, manual 97-test, endpoint and final invoice/gas 205-test runs passed. No claim that the entire 351-file suite was rerun after every late fix. |
| Frontend lint/types/build | Passed; five pre-existing lint warnings, zero errors. Build used a synthetic public WalletConnect ID and `VITE_DEV_MODE=false`; production guards remain active. |
| Optimized wasm | Built using the repository optimizer with a named target overlay. Reviewed immutable token/router hashes below match actual LocalTerra stores. |
| Isolated LocalTerra | Six success/rollback fixtures passed, then passed again with mixed **18/6** decimals. Actual legacy→v1 router/token migrations, preserved balances/config/pools and pair-pin refreshes passed. |
| Atomic composite flows | Actual taxed-route→50-UST1 feature invoice, wrong-invoice atomic rollback, native wrap→taxed route, wrap/minimum-return rollback and taxed-route→native unwrap passed. Only synthetic LocalTerra test-keyring accounts/funds used. |
| Legacy verifier updates | #607/#615/#1267/#1285/#1280/#623 checks distinguish v1 from legacy behavior. Shell syntax and #623 helper/mixed-version/accounting regressions passed; legacy direct-pair/SendFrom assertions remain covered by contract tests. |

The verifier runs contract/indexer/frontend checks and retests, then lint/types/build
and real chain execution. It permits only explicit unavailable-check skips; a
failed executed assertion remains FAIL. All changed indexer Rust files pass
formatting. A whole-tree diagnostic found unrelated pre-existing formatting drift;
the verifier checks every changed file without rewriting those legacy test files.
An auxiliary strict all-target Clippy diagnostic likewise found pre-existing test
harness style warnings; standard workspace production Clippy passes.

## Deployed test-chain identity and measurements

These are **LocalTerra**, not Columbus-5 stores:

| Artifact | Code | SHA-256 |
|---|---:|---|
| Router 1.2.0 | 62 | `72f1938ee3868ee22425d9237cb3b1b41b9b3f89df98c891dc40a85591f182d7` |
| Community-tax token 1.2.0 | 63 | `1beecb175617ca707ac1577597d68ad0576d6f78b75ec7006bc904444c58b6d0` |

Mixed-decimal fixture measured gas:

| Flow | Gas used |
|---|---:|
| CL8Y→ALPHA→UST1→USTR, zero intermediate holdings | 4,413,139 |
| CL8Y→ALPHA→BETA→USTR, two taxed intermediates | 4,530,025 |
| Same three-hop flow with router dust/pre-existing holdings | 4,419,113 |
| First-hop ALPHA gross budget, two hops | 3,077,463 |
| Final taxed buy, one hop | 1,872,341 |
| Minimum-return failure, entire token/tax route rolled back | 4,153,011 |

V1 static fallback floor is `600,000 + 1,700,000 × hops`, plus existing book-walk
excess and native wrap/unwrap message costs. It exceeds **1.2×** all the measured
pool examples. Actual signing still prefers the existing **1.2× LCD simulate**.
These measurements are not a Columbus-5 gas census.

The original example reconciles gross **1,010,000** ALPHA to economic **1,000,000**
plus tax **10,000**. Route buys fund the next gross sell budget with net proceeds.
The composite invoice delivered exactly **50,000,000** UST1 to its configured
feature receiver and kept excess with the payer. Native unwrap minimum is
mapper-net before Classic burn; router arithmetic output is net CW20 before
mapper fees, preserving the existing native-fee convention.

## Acceptance criteria

- [x] **AC1:** actual tax-on LocalTerra zero-intermediate and multi-tax routes.
- [x] **AC2:** route-funded buy/sell boundaries; trader controls and exact sink charges.
- [x] **AC3:** economic pair credit, tax/supply/sink ledger, net quotes, reverse sizing and owned residue.
- [x] **AC4:** authorized gross input only; existing wallets, allowances and router dust cannot subsidize tax.
- [x] **AC5:** supported intermediate eligible/net-ranked; unsupported versions fail explicitly.
- [x] **AC6:** funded GET/POST/manual hybrid/exact-output/progress/cache and fallback parity within the tested paths.
- [x] **AC7:** gross Swap/Trade/Max/pay/receive/invoice/native budgets and failed-discovery/balance protections; real atomic invoice/wrap/unwrap checks.
- [x] **AC8:** zero rates/exemptions and legacy direct-pair compatibility preserve guard/security regressions.
- [ ] **AC9, production portion:** actual LocalTerra migrations/capabilities are verified; Columbus-5 store/migrate/whitelist/pin refresh/cache/frontend rollout and live execution are not performed.
- [x] **AC10:** route and composite invoice/native failures roll back atomically; distinct funding/balance recovery, no intermediate-reserve advice or automatic resend.

## Unexecuted checks and follow-up

Read-only Columbus-5 evidence at block **30686762**, **2026-10-04T10:05:04 UTC**:
official router code **11640** did not advertise funding v1; ALPHA **11666** rejected
`route_tax_capability`. [Read-only production inventory](columbus5-readonly.json). No production signing, funds, migrations or successful live
execution is claimed. DEX multisig/CMM signing custody was unavailable to this agent.
Follow the coordinated runbook when authorized custody/release execution is available,
then replace AC9's skip with actual code/hash/capability and execution receipts.

The full historical shared-stack Layer B #623 suite was not rerun against the
old deployment. Its scripts are version-aware; isolated current tax-on executions
and contract regressions provide the implementation evidence. Reporter transaction
details and physical wallet/device replay were unavailable and are not invented.
Agents may record these as explicit skips while reporting overall source verification
**PASS with explicit skips**; skipped production acceptance remains unverified.
