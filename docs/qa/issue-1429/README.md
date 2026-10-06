# Issue #1429: inventory, recovery and agent verification

[#1429](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1429) covers legacy
AutoLP policy recovery through the actual manager. Read
[LP1429-1–LP1429-8](../../../skills/AGENTS_AUTOLP_POLICY_RECOVERY.md) and
[NE1398-9](../../../skills/AGENTS_NET_EXECUTION_FLOORS.md).
Token **1.4.0** adds `RefreshAutolpPolicy`; the sister remains reviewed **1.1.0**.
Shared `LpPolicy` fields, JSON and storage keys are unchanged.

## Reproduce source checks

```bash
make verify-issue-1429
make verify-issue-1398
make verify-issue-1237
```

Optional `CONTRACTS_CARGO_HOME` / `CONTRACTS_CARGO_TARGET_DIR` select compatible
dependency/build caches. Run against this worktree's source, inspect assertions
and record exact command, revision, UTC timestamp, counts and outcome. The #1429
verifier repeats recovery/economic/no-op tests, runs the full contract workspace,
checks formatting and documentation links. Retest after corrections.

## Issue criteria and agent checklist

- [x] **AC1 — inventory:** Run the read-only inventory script below; inspect each
  bound sister's actual manager and policy. Distinguish null, unsupported and
  failed queries. Record code-list boundaries, pagination and endpoint/height.
- [x] **AC2 — manager execution:** Verify token manager can refresh its bound
  token-owned sister with exact positive live reserves/share/LP identity. Reject
  foreign callers, direct human impersonation, wrong sister identity/factory,
  changed pair/quote/LP, zeros, changed shares/reserves, lock and missing feature.
- [x] **AC3 — unchanged settings:** Verify identical settings reject the invoice,
  retain payer/CMM balances and leave a missing anchor missing. Explicit recovery
  preserves sister settings and requires no settings invoice or attached funds.
- [x] **AC4 — economic floors and rollback:** Verify missing/stale anchors block
  permissionless skim, which never refreshes policy. Verify shortage and failed
  swap/provide replies restore token/quote/LP balances, pool/shares, allowances,
  pending state and lock. Verify downstream registration failure also rolls back
  recovery, then a corrected retry succeeds.
- [x] **AC5 — review and retest:** Independent agent review, focused tests twice,
  workspace regression, formatting and crosslinks pass; correct and retest every
  executed failure. Inspect migration/version behavior and unchanged storage.
- [x] **AC6 — isolated wasm/chain:** Build optimized wasm and deploy to a private
  LocalTerra stack when available; reproduce recovery and failed-reply readback.
  Attempt bounded provisioning before declaring infrastructure unavailable.
- [ ] **AC7 — production migration/readback:** Record actual stored/migrated token
  and sister code IDs, cw2 versions, transaction receipts and exact recovered
  `GetLpPolicy`. Do not infer this from source tests. If no bound sister exists in
  the observed inventory, record that fact; no recovery transaction is possible.
  Production enrollment and maximum-hook gas remain
  [#1421](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1421).

## Actual-manager recovery procedure

1. Inventory launcher/token/sister code and configuration using
   `python3 scripts/qa/inventory-autolp-1429.py`. Re-run on the intended endpoint;
   [the saved receipt](inventory.json) is an observation at its recorded height.
2. If the sister manager is a wallet, that wallet explicitly executes the sister's
   same-pair `UpdateConfig`. If it is this bound token, migrate the token to reviewed
   **1.4.0** and sister to policy-aware **1.1.0** through their actual wasm admins.
   Record migration receipts separately. The proxy never grants migration rights.
   After token migration, refresh the listed pair's cached asset code IDs through
   the authorized factory `RefreshPairAssetCodeIds` path before skim/enrollment;
   preserve the existing freeze gate instead of bypassing it.
3. Query sister config/policy and pair `Pair`/`Pool`; prepare an explicit snapshot.
   The token manager executes the following on the **token**, with no funds:

   ```json
   {"refresh_autolp_policy":{"expected_policy":{"pair":"terra1...","quote":"terra1...","lp_token":"terra1...","tax_reserve":"1000000","quote_reserve":"2000000","total_share":"1000000"}}}
   ```

   Amounts above illustrate the shape; use reviewed **actual raw live values**.
   Never approve a manipulated/empty pool or reuse a stale snapshot. Exact mismatch
   fails and requires a new explicit manager decision. No auto-retry with a newly
   fetched spot anchor. Pair changes and other settings use their existing paths.
4. Read back `GetLpPolicy` and require exact snapshot equality; read back config
   and compare all unrelated fields. Record block/tx/code/version independently
   before enrolling the sister. Unsupported policy query requires migration first.

## Isolated wasm/chain reproduction

Provision a uniquely named LocalTerra container and owned volume using the pinned
image in `docker-compose.yml` and this checkout's `docker/init-chain.sh` mount.
No host ports are needed. The smoke's Docker-exec helper rejects every network
except `localterra`; it never starts, resets or removes containers itself. Use
synthetic test keys only and clean up only resources created for this run.

Build current wasm with the compatible **Rust 1.81.0** target toolchain, then
optimize each artifact with **wasm-opt 116** `-Oz --signext-lowering`. The observed
build command, from `smartcontracts`, was:

```bash
CARGO_HOME=/tmp/cl8y-1398-cargo-home CARGO_TARGET_DIR=/tmp/issue1429-wasm81-target RUSTC=/home/answorld/.rustup/toolchains/1.81.0-x86_64-unknown-linux-gnu/bin/rustc RUSTFLAGS='-C target-feature=-reference-types,-multivalue,-bulk-memory -C link-arg=--allow-undefined' cargo build --locked --release --target wasm32-unknown-unknown -p cl8y-community-tax-token -p cl8y-community-tax-autolp -p cl8y-dex-factory -p cl8y-dex-pair --lib
```

Adjust toolchain/cache/output paths to the agent's environment. Place optimized
current artifacts and the reviewed `cw20_mintable.wasm` fixture in one artifact
directory. Obtain baseline token **1.3.0** matching the
[reviewed build manifest](../merge-2026-10-06-final/build-provenance.json), plus
historical AutoLP code **11633** bytes through read-only LCD code retrieval. The
smoke verifies their hashes and cw2 versions, each stored code hash and migrated
versions. Do not substitute a different artifact merely because its version
string matches. Missing reviewed artifacts may be an explicit SKIP.

Run the [focused smoke](../../../scripts/qa/localterra-autolp-recovery-1429.py)
twice on fresh synthetic fixtures:

```bash
python3 scripts/qa/localterra-autolp-recovery-1429.py \
  --container YOUR_OWNED_LOCALTERRA \
  --artifacts /path/to/current-optimized-artifacts \
  --legacy-token /path/to/reviewed-token-1.3.0.wasm \
  --legacy-token-provenance docs/qa/merge-2026-10-06-final/build-provenance.json \
  --legacy-autolp /path/to/historical-autolp-11633.wasm \
  --evidence-file /path/to/new-receipt.json
```

Optional `--code-cache /path/to/cache.json` reuses checksum-verified code IDs on
the **same owned chain**; use a new cache for a newly provisioned chain. Inclusion
is bounded by `--inclusion-timeout` (default 180 seconds, maximum 300). An uncertain
accepted upload/transaction requires reconciliation by tx hash, never a blind
rebroadcast. Rejections must match the expected contract error and preserve full
CW20/LP economic snapshots; native network fees are deliberately outside those
snapshots. Source multitests separately cover insufficient LP and failed provide
replies, including complete rollback and corrected retries.

## Explicit skip policy

Complete all runnable work first. An agent may mark an unavailable infrastructure,
signing, deployment, device or other row **SKIP** with its observed reason and
still mark overall **PASS WITH SKIPS**, provided every executed check passes.
Leave skipped boxes unchecked; no holds or gates are created by omitted checks.
Executed failures remain **FAIL** until corrected and retested. Skipping does not
establish omitted acceptance or production deployment. The source verifier accepts
`VERIFY_ISSUE_1429_SKIP_CONTRACTS_REASON`, `..._FULL_CONTRACTS_REASON`,
`..._FORMAT_REASON`, `..._DOCS_REASON` with explicit observed reasons.

## Evidence receipt

Source verified on **2026-10-06 at 07:13 UTC**, against base
`e7b54989c21fc411141c4c6c3af553c03dea1bdd` plus this PR's implementation in
`fix/issue-1429-manager-policy-recovery`. Contract-source SHA256 is
`b9cebba5b8d2ca73c56e19d28cc4d80287b2f6c7f2e34e52c624068cf0e6084e`
over the 165 sorted tracked `smartcontracts/**/*.rs`, `Cargo.toml` and `Cargo.lock`
source files (excluding build outputs), ordered by path components with Python
`sorted(Path(...))`,
hashing each relative path, NUL, contents and NUL. Checked AC1–AC5 represent
inventory and source/multitest evidence; deployment remains separate.

Exact source commands (cache overrides are optional):

```bash
CONTRACTS_CARGO_HOME=/tmp/cl8y-1398-cargo-home CONTRACTS_CARGO_TARGET_DIR=/tmp/issue1429-test-target CARGO_BUILD_JOBS=2 make verify-issue-1429
CONTRACTS_CARGO_HOME=/tmp/cl8y-1398-cargo-home CONTRACTS_CARGO_TARGET_DIR=/tmp/issue1429-contract-target make verify-issue-1398
CARGO_HOME=/tmp/cl8y-1398-cargo-home CARGO_TARGET_DIR=/tmp/issue1429-test-target CARGO_BUILD_JOBS=2 make verify-issue-1237
CARGO_HOME=/tmp/cl8y-1398-cargo-home CARGO_TARGET_DIR=/tmp/issue1429-contract-target CARGO_BUILD_JOBS=2 cargo clippy --manifest-path smartcontracts/Cargo.toml --workspace --all-targets -- -D warnings
python3 scripts/qa/test_inventory_autolp_1429.py
```

| Check | Outcome |
| --- | --- |
| #1429 focused recovery, initial/retest | PASS: 14 cases each; complete token 128 and AutoLP 29 tests each in the verifier |
| #1429 source verifier | PASS: 5 PASS, 0 FAIL, 0 SKIP |
| Full contract workspace | PASS: 996 tests in each source verifier |
| #1398 net-floor verifier | PASS: 7 PASS, 0 FAIL, 0 SKIP; frontend 168 tests each run |
| #1237 unchanged-settings verifier | PASS: 8 PASS, 0 FAIL; 13 no-op contract, 29 AutoLP and 22 frontend tests each run |
| Strict workspace Clippy, all targets | PASS with `-D warnings` |
| Rust formatting / documentation crosslinks | PASS |
| Offline inventory pagination / query classification | PASS: 14 tests, repeated |
| Verifier accounting | PASS: explicit unavailable groups yield PASS WITH SKIPS; injected executed failure remains FAIL/nonzero |
| Independent security and regression review | PASS: no unresolved security findings; required `GetLpPolicy` preflight rejects unsupported sister versions |
| Optimized wasm and isolated LocalTerra, first/retest | PASS: compatible Rust 1.81.0 build and wasm-opt 116; each fresh fixture passes 10 checks with 23 transaction receipts |

The [read-only production inventory](inventory.json) completed at
**06:57:53–06:58:01 UTC**, endpoint-observed Columbus-5 block **30714032**.
Historical/current AutoLP codes **11613, 11621, 11633** each exhausted their
contract list with no instances. The two discovered tokens (codes **11630** and
**11666**) both have `autolp: null`. There was consequently no sister address
whose manager/policy could be read or recovered within this documented scope.
The script also follows actual token bindings when present; this inventory is
not a claim that every chain code ID was inspected.

Production migration/recovery/readback is **SKIP / not applicable to the observed
unbound tokens**: no target sister exists in this inventory, and no production
transaction was submitted. This leaves AC7 unchecked. Future rollout and maximum
hook gas remain #1421; source passes are not production acceptance.

The accepted [first chain receipt](localterra-first.json) covers
**07:13:27–07:15:11 UTC**, and the independent [retest](localterra-retest.json)
covers **07:15:50–07:16:52 UTC**. They record all stored code hashes, baseline
artifact provenance and actual cw2 readback of token **1.3.0 → 1.4.0** and AutoLP
**1.0.0 → 1.1.0**. Both recovered the exact approved policy, kept unrelated settings
unchanged and minted **498500 raw LP** through a permissionless skim without
refreshing the anchor. The expected failed swap preserved pool/shares, all CW20/LP
balances and supplies, allowances, token/sister config, policy, pending state and
lock; rejections matched their actual expected contract error.

Smoke script SHA256:
`63f78c62623dc5cc68d8d937fb4efbf52a08c74db5351f90b5b4a5b91c11f079`.
Initial Rust 1.99 wasm attempts exposed incompatible linker/VM features; the
compatible Rust 1.81.0 library build and optimization succeeded. Initial fixture
CLI integer arguments and a non-alphanumeric synthetic token name were corrected
before the accepted runs. A stalled first upload was reconciled by transaction
hash after restarting only the owned node; it was never blindly rebroadcast.
No executed failure was relabeled as SKIP. The dedicated container and volume
were removed after successful retest; production/shared resources were untouched.

Overall agent verification is **PASS WITH SKIPS** for the explicitly omitted
production scope; source and synthetic-chain checks themselves are **PASS**.
