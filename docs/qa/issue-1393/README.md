# Issue #1393 agent review and verification

[Issue and comments](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1393)
cover Bridge-Debt default rotation, mutable identities, migration and owner risk,
issuance limits and residual governance powers.
[BD1393-1–BD1393-10](../../../skills/AGENTS_BRIDGE_DEBT_MARKET_PINS.md) are normative;
the [USDT/UST1 runbook](../../runbooks/deploy-usdt-ust1-lp.md) explains operator messages.
Record your run in [verification.md](verification.md).

1. Read the issue/comments and linked invariants; inspect both changed crates and
   the deployment script. Use an isolated git worktree and human git identity.
2. Build the two optimized contracts and pinned CW20 fixture with
   `bash scripts/qa/build-issue-1393-wasm.sh` (install Rust 1.81 and its wasm32 target:
   `rustup toolchain install 1.81.0 --target wasm32-unknown-unknown`; `wasm-opt`
   and modern Cargo supporting the v4 lockfile are required), or the standard
   workspace optimizer. The narrow helper uses the 1.81 compiler through `RUSTC`
   with modern Cargo; current prebuilt Rust wasm std emits unsupported bulk-memory
   instructions even with crate feature flags disabled. Actual compiler, Cargo,
   optimizer versions and flags are recorded in `issue1393-build.json`.
   The narrow build uses the
   `cw20-mintable` revision from `smartcontracts/Cargo.lock`, locates its standalone
   manifest with locked Cargo metadata, and builds against its committed lockfile.
   Alternatively set `VERIFY_ISSUE_1393_CW20_ARTIFACT` to an existing checksum-reviewed
   optimized fixture. The chain evidence records the actual artifact SHA256.
   Run `make verify-issue-1393`.
   It runs focused tests twice, clippy, formatting,
   shell regression checks, documentation links and optimized LocalTerra smoke
   twice. The chain smoke creates standalone synthetic contracts on localterra;
   it does not reset the shared chain or use production keys.
3. Review the criteria below, mapping each to concrete tests and receipts. Inspect
   failure snapshots, rather than assuming a rejected message means rollback.
4. Broaden to `make test-contracts` when feasible. For deployed acceptance, collect
   read-only code/checksum/admin/minter/custody evidence for every live market.
   Production execution is separate and must have actual transaction evidence.
   [Existing production read-only baseline](live-baseline.json) records code hashes,
   multisig threshold and accounting checks without claiming an upgrade deployed.
5. Record PASS/FAIL/SKIP, commands, commit, wasm SHA256, chain heights and reasons.
   Do not check off a criterion supported only by a skipped test.

| Criterion | Agent action and expected result |
|---|---|
| AC1: independent pins | Rotate default receipt code, approve another underlying and register second market. Issue/Repay original valid market and second market; each keeps its own receipt/code pins. |
| AC2: mutation and upgrade atomicity | Attempt unauthorized receipt minter replacement, underlying code/minter/cap drift, mint/burn tampering, wrong receipt and inconsistent supply. Compare outstanding, supply and all touched balances before/after failures. Propose reviewed code-only upgrade, migrate children and accept in one admin transaction; identities, supply and repayment must persist. |
| AC3: migration and ownership | Try wrong cw2 contract, malformed/newer source version, missing/duplicate/extra legacy pins and invalid legacy accounting; reject without writes. Migrate supported current/legacy storage with reviewed full pins and preserved maps/accounting. Ownership changes only after recipient acceptance; test cancellation and unauthorized acceptance. |
| AC4: finite issuance and policy | Test registration disabled, exact cap boundaries, over per-issue/outstanding/epoch limits, overflow and fixed-epoch rollover; configure/pause/repay cannot reset quota. Valid Repay and Recover remain usable during issuance pause, without granting holder receipt burn/transfer. Review documented residual admin powers and display-only valuation. |
| Integration | Dry-run and offline deploy-script regression: approve exact LCD USDT code as multi-minter, set finite 10k raw-unit caps before Issue, refuse existing mismatched pins/limits, retain resumed issuance protection. Optimized wasm Issue/Repay smoke twice; compare raw units and receipts. |

## Explicit skip policy

Complete every available check first. If a dependency, device, privileged key,
external service or chain environment is unavailable, record **SKIP** with its
reason, attempted command and precise unverified scope. Set
`VERIFY_ISSUE_1393_SKIP_<GROUP>_REASON` for `TESTS`, `SCRIPTS`, `STATIC`, `CHAIN` or `FULL`
as appropriate. The summary can be **PASS with skips** when all executed checks
pass, even if unavailable checklist items remain unchecked. An executed failure
remains **FAIL** until fixed and rerun. Never convert a failure to a skip.
Skipped production/deployed checks do not establish production deployment,
custody correctness or live migration acceptance. Relevant operator followups
belong in one-action cl8y-pm inbox cards when human-only work remains.
