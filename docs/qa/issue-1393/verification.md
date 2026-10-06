# Issue #1393 verification record

Implementation verification: **PASS** — final `make verify-issue-1393` completed
with **PASS=9 FAIL=0 SKIP=0** on 2026-10-05T08:00:34.358969+00:00. Production migration is unverified;
these checks establish source and synthetic execution acceptance.

Base revision: `9944ada10ee711ff8ffaa91f4c19b0f292de3924`.
Branch: `fix/issue-1393-market-pins`; the PR commit records the tested change.
[Exact source SHA256 inventory](source-sha256.json) fixes the tested code independently
of subsequent documentation edits. [Agent procedure](README.md),
[BD1393-1–BD1393-10](../../../skills/AGENTS_BRIDGE_DEBT_MARKET_PINS.md),
[runbook](../../runbooks/deploy-usdt-ust1-lp.md).

- [x] AC1: default rotation and second-market registration preserve per-market Issue/Repay.
- [x] AC2: rejected identity/accounting mutations are atomic; approved child upgrades preserve supply, debtor, outstanding and repayment.
- [x] AC3: wrong/newer migrations reject; reviewed current/legacy storage preserves state; owner recipient accepts.
- [x] AC4: finite per-issue/outstanding/epoch limits, immutable epochs, repay/recovery policy and residual governance powers verified.
- [x] Focused crate tests pass and pass again: **23 issuer + 12 receipt tests** per run.
- [x] Full contract suite passes: **901 passed, 0 failed, 0 ignored**; repeated by the final verifier.
- [x] Clippy with warnings denied, touched-crate formatting, shell syntax, **7 deployment regressions** and documentation links pass.
- [x] Optimized standalone LocalTerra Issue/Repay/upgrade smoke passes twice with artifact hashes and actual receipts.
- [x] Live read-only code/admin/minter/custody inventory and complete one-market supply/debtor/outstanding reconciliation collected.
- [ ] Production migration/deployment acceptance: **not performed**, outside this implementation PR; live baseline remains issuer/receipt 1.0.0.
- [ ] Optional integrated synthetic pool dump/price-impact simulation: **not run**. The standalone fixture contains issuer/receipt/CW20 contracts, with no pool venue fixture. Caps were verified in raw units; no dollar-loss or price-impact bound is asserted.

## Executed commands and scope

```sh
cargo test --offline --workspace --locked -j 2 --manifest-path smartcontracts/Cargo.toml
cargo test --locked --manifest-path smartcontracts/Cargo.toml -p bridge-debt -p bridge-debt-token -- --test-threads=1
cargo clippy --locked --manifest-path smartcontracts/Cargo.toml -p bridge-debt -p bridge-debt-token --all-targets -- -D warnings
cargo fmt --manifest-path smartcontracts/Cargo.toml -p bridge-debt -p bridge-debt-token --check
python3 scripts/qa/test-deploy-usdt-ust1-lp1393.py
python3 scripts/qa/check-issue-1393-docs.py
bash scripts/qa/build-issue-1393-wasm.sh
VERIFY_ISSUE_1393_CONTAINER=cl8y-issue1393-localterra make verify-issue-1393
```

Native checks used Cargo's cached registry/git sources via an isolated `CARGO_HOME`
and two build jobs. The final verifier used a fresh code cache and separate evidence
prefix. Both the explicit reused CW20 fixture and the default locked standalone
CW20 build were compiled/stored/executed through their supported paths. Native
unit tests include two real synthetic receipt code IDs, wrong minter/code/decimals/
supply, clamped minter pagination at 30/31, owner propose/cancel/accept, true legacy
serialized layout, zero/overflow, partial/full repayment and recovery. Mint and
second-leg burn failures compare complete ledger/balance/quota/pending snapshots;
ten malformed migration cases compare byte-for-byte storage before/after.

## Optimized on-chain evidence

[First final run](localterra-first.json): 31 receipts,
heights 12427–12916.
[Final retest](localterra-retest.json): 28 receipts,
heights 12951–13331.
[Additional reused-fixture run](localterra-reused-fixture.json) exercises the
approved historical underlying artifact by actual checksum; no source-match
certification is inferred from reuse.

| Artifact | SHA256 |
|---|---|
| Issuer | `02e3705e86544c04e583c15802b69b800082222642a9a840a09ba56847d25cc2` |
| Receipt | `d570c2f2a3863946c6d0627c24850875fb26d7878c8887ca3b744336d60607b7` |
| Default standalone CW20 fixture | `c12f344035f8520699905e8f86cf3bff6e2ba0b90c8303fec5d3bb9bfe788f84` |

Build: Rust **1.81.0**, modern Cargo **1.99.0**, Binaryen **116**;
`-Oz --signext-lowering`, exact Rust flags/toolchain in the JSON receipts. The
shared LocalTerra stopped confirming a submitted store, so it yielded no contract
execution evidence. A separate SDK53 LocalTerra with synthetic genesis, 1 GiB tmpfs
and no published ports supplied all PASS receipts without resetting shared state.

Initial Rust 1.99 artifacts failed real VM static validation for bulk-memory
instructions, including after crate-level feature disabling. Those failures were
fixed by the pinned 1.81 Wasm compiler/standard library and rerun successfully;
they were not relabeled as skips. The production [read-only baseline](live-baseline.json)
uses independent latest-state queries, not one atomic block snapshot. It reports
actual multisig threshold 2-of-3 without claiming the source enforces signer count.

Agents may mark an unavailable item SKIP with attempted command, reason and
unverified scope, and report **PASS with skips** when every executed check passes.
Skipped criteria stay unchecked; executed failures remain FAIL until corrected
and rerun. This permission never establishes production acceptance.
