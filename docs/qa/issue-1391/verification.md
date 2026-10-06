# #1391 verification receipt

Issue [#1391](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1391).
Implementation: [PR #1412](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1412),
[revision 421ef59f](https://git.cl8y.com/code/cl8y-dex-terraclassic/src/commit/421ef59f3a1978e9cd5f88c76ecbf4c90eaad925),
branch `fix/1391-oracle-integrity`, based on
`9944ada10ee711ff8ffaa91f4c19b0f292de3924`. Publication-link updates do not change the verified implementation.
[Policy/checklist](README.md), [O1391 invariants](../../indexer-invariants.md#oracle-integrity-and-hub-containment-1391),
[third-party agent instructions](../../../skills/AGENTS_ORACLE_SAFETY.md).

Final verification: **overall PASS with explicit skips**, 2026-10-05 UTC (aggregate completed before 08:58:54 UTC; final identity repair retest completed after 09:01:24 UTC).
Aggregate report: **PASS=6 FAIL=0 SKIP=2**. All executed final assertions passed;
no failed assertion was waived. Synthetic identities, mocked HTTP/signing and a
separate Postgres container/database were used. No production manipulation,
deployment, signing or transaction acceptance is claimed.

## Exact aggregate command and outcomes

From the implementation worktree root:

```bash
CARGO_HOME=/tmp/cl8y-dex-1391-cargo-home \
CARGO_TARGET_DIR=/tmp/cl8y-dex-1391-indexer-target-nodebug \
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 \
VERIFY_ISSUE_1391_SKIP_CHAIN_REASON='Shared inclusion timed out; independently provisioned private chain remained stale/app not ready and bounded restart failed (docs/qa/issue-1391/chain-probe.md).' \
VERIFY_ISSUE_1391_SKIP_LIVE_REASON='PR has not been deployed; no production acceptance or manipulation performed.' \
make verify-issue-1391
```

The `/tmp` Cargo paths isolate build/registry locks on the shared host; other agents
can choose their own paths or omit these two variables. Debug symbols were disabled
to reduce memory use; test assertions and overflow checks remained enabled.
`indexer/.env` selected only the isolated test DB, with a separate
`TEST_DB_LOCK_FILE`. Credentials/runtime files were not committed. Both passes ran
from `indexer/` so the isolated environment loaded before DB access.

| Suite | First pass | Retest |
|-------|------------|--------|
| Oracle parser/quorum/cache unit filter | 29 passed | 29 passed |
| Hub/identity/reference/expiry unit filter | 31 passed | 31 passed |
| Economic mark unit filter | 9 passed | 9 passed |
| Protocol TVL unit filter | 20 passed | 20 passed |
| Hub API and canonical reserve attacks | 7 passed | 7 passed |
| Oracle API, storage bounds/replay/schema | 23 passed | 23 passed |
| Human candle USD | 2 passed | 2 passed |
| Idle/as-of candle USD, stale/uncertified/spoof repair | 11 passed | 11 passed |
| DeFiLlama adapter | 10 passed | 10 passed |
| Protocol liquidity | 29 passed | 29 passed |
| Protocol liquidity series | 7 passed | 7 passed |
| Human swap USD | 2 passed | 2 passed |
| Volume USD catalog | 6 passed | 6 passed |
| Python price/Venus/shape safety | 7 passed | 7 passed |
| Python financial consumers and actual shell entry points | 3 groups passed | 3 groups passed |
| Buyback quote/oracle/retry failures and healthy controls | 9 passed | 9 passed |
| LP, buyback and rebalance self-checks/shell checks | passed | passed |

Aggregate totals: **186 Rust tests per pass, 372 executions**; **19 Python test cases per
pass, 38 executions** (consumer cases contain additional subtests). Whitespace
and documentation-link checks passed. Node **24.21.0** TypeScript validation also
passed with `bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/tsc -b --pretty false`.
Independent agent review found no blocking finding; its reported edge cases were
fixed and included in the final retests.

Earlier test failures were fixed and retested: nonfresh fixture certificates had
to clear accepted sources/as-of, liquidity fixture row arity needed correction,
and healthy candle fixtures needed real policy confirmation/identity rather than
legacy symbol trust. A source-pair deletion TTL extension and candle symbol spoof
were corrected in implementation. Initial mock policy/retry-helper fixtures and
a fixture borrow lifetime were also corrected. Final independent review additionally identified mixed native/CW20 flags in historical repair. Both repair queries now require consistent flags and exact identity. After that narrow final fix, `cd indexer && cargo test --test candle_usd_mark -- --test-threads=1` passed **11/11 twice**, including both mixed-flag attack variants (22 additional Rust executions; **394 total**). Independent delta review then passed. Interrupted/abandoned compilation
runs were not counted as test passes.

## Completed criteria and remaining scope

- [x] **AC1:** Invalid numeric/source inputs are rejected before new persistence;
  quorum/outlier policy, immediate failure clearing, age bounds, future replay,
  failed publication, legacy cache poisoning and recovery are tested.
- [x] **AC2:** Synthetic actual-canonical-pair reserve shifts, priced-leg depth
  loss, USTR ties/pinning/switches and UST1 error propagation are contained under
  the documented fixed reference; API/adapters disclose advisory provenance.
- [x] **AC3:** All six repository indexer-fed financial actions validate source,
  age and quality before preparation. Actual admin/multisig/generate/sign
  functions were exercised with delayed unhealthy responses and healthy controls;
  unsafe responses never reached the mocked transaction helper.
- [x] **AC4:** Exact identity, mixed decimals, coverage, historical repair and
  API/adapter regression suites pass. Policy/invariants/agent docs are crosslinked
  and disclose remaining spot manipulation and provider-replay risk.
- [ ] **Real chain attacks: SKIP/unverified.** Shared-chain funding inclusion timed
  out. An independent chain was provisioned but remained stale/app-not-ready;
  its bounded restart/liveness retry also failed after memory became available.
  Exact timestamps, conditions and reproducible harness/replay instructions:
  [chain-probe.md](chain-probe.md). No real reserve-attack receipt exists; the
  optional replay test was not counted as a passing chain test.
- [ ] **Production deployment acceptance: SKIP/unverified.** This is a PR, not a
  deployed revision. Migrations, public endpoints and live automated actions have
  not been accepted in production. No contract migration is required.

Fresh CEX quality proves recent **local HTTP observation** and source agreement,
not authenticated market time or fair value. Hub marks remain non-oracle-grade;
manipulated bootstrap, within-bound changes and coordinated provider/pool replay
remain possible. Fixed references require reviewed maintenance for intentional
repricing; no automatic ratchet/reset is provided.

Agents may mark overall verification **PASS with explicit skips** for unavailable
items after completing runnable checks. Record the attempted prerequisite and
reason, keep skipped criteria unchecked, and fix/retest executed failures. The
full reproduction/review instructions are in [README.md](README.md); verification
is assigned to agents, not the human.

Verified code patch SHA-256 (against the base above, paths `indexer`,
`frontend-dapp`, `scripts`, `Makefile`; excludes documentation-only publication links):
`510d7d2c19f94f5058163304f580e2817ad93b88066e3622fe8c822edd6b6cf4`.

Additional publication checks: contract formatting/Clippy, staged frontend ESLint/
Prettier and TypeScript passed. The secret scanner initially flagged three
lowercase copies of the public pinned Venus contract: the existing allowlist used
only its checksummed spelling. Its exact-address rule is now case-insensitive;
`gitleaks protect --staged -c .gitleaks.toml --redact` passes with no leaks. No broad
address/path exemption or commit-hook bypass was used.
