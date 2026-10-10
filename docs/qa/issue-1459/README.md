# Issue #1459 implementation and agent verification

Implementation: [DEX PR #1463](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1463)
and [companion ustr-cmm PR #62](https://git.cl8y.com/code/ustr-cmm/pulls/62).


Issue and comments: [#1459](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1459).
Baseline implementation: [#1458](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1458).
Invariants: [QC1459-1–QC1459-10](../../../skills/AGENTS_LOCAL_QUOTE_COVERAGE.md),
[LQ1452](../../../skills/AGENTS_LOCAL_SWAP_QUOTES.md),
[funding](../../../skills/AGENTS_ROUTE_LOCAL_TAX.md),
[floors](../../../skills/AGENTS_NET_EXECUTION_FLOORS.md) and
[launch guards](../../../skills/AGENTS_ROUTE_LAUNCH_GUARDS.md).
Companion protocol: [ustr-cmm PR #62](https://git.cl8y.com/code/ustr-cmm/pulls/62)
(`519c2766e5be0a3788a4c920ad879b641a50aa74`).
Exact completed checks and remaining scope: [RESULTS.md](RESULTS.md).

## Implementation map

- [Scheduled wrapper observation](../../../indexer/src/api/local_wrapper_quote.rs)
  reads `execution_capabilities {}` outside requests. Only the integer
  `direct_wrapper_floor_version:1` is admitted. Wrap needs treasury and mapper;
  unwrap needs mapper. Unsupported deployments retain positive estimates and
  a single signing blocker. Generations include both capability observations.
- [Zero-hop modes](../../../indexer/src/api/local_quote/modes.rs) pin
  `wrapper.floor_protocol_version:1`, exact input, positive wallet floor and
  capability-dependent execution support. [Frontend validation](../../../frontend-dapp/src/utils/localWrapperOnlyQuote.ts)
  and [preparation](../../../frontend-dapp/src/utils/localRouteQuote.ts) bind
  that metadata to the explicit protected external messages. The wallet receive
  floor is used for direct unwrap, even when the bank-transfer base differs.
- [Scheduled admission](../../../indexer/src/api/local_quote/admission.rs),
  [wallet selection](../../../indexer/src/api/local_quote/ingestion.rs) and
  [tax observations](../../../indexer/src/api/local_quote/tax.rs) admit bounded
  `LOCAL_QUOTE_WALLETS` enrollment and CW20 holder pages. Every actor still needs
  blacklist, protocol/manager/exemption and required balance observations.
- [Composed inverse](../../../indexer/src/api/local_quote/reverse.rs) recomposes
  necessary lower bounds and checks the full forward path. A failed composition
  advances to the next first-hop original pool-output event without assuming
  post-hook monotonicity. Bounds are 64 paths, 64 whole-path refinements,
  64 refinements per hop and 512 shared hop probes per candidate path.

## Scheduled wallet budgets

`LOCAL_QUOTE_WALLETS` is a validated, deduplicated comma-separated list of at
most 2,048 Terra addresses. Four rotate per independent pass, including buyers
with no balance or indexed trades. Four strictly ordered CW20 holders per tax
token rotate by retained pagination cursor; malformed or failed pages do not
advance it. These join eight recent and eight historical-negative observations,
with at most 24 actors per token observation. A compatible per-token wallet ledger survives pair replacement and completed
holder pages; old pair registration/config times are never inherited with it.
Known factory negatives publish synchronously before DB/token enrichment, including
a bounded cold guard ledger. Complete epoch-checked inventory is needed to clear
quarantine; partial enumeration cannot clear either factory or protocol-negative
capacity state. The [token role guard ledger](../../../indexer/src/api/local_quote/protocol_guards.rs)
retains at most 2,048 distinct `(token, wallet)` negatives per namespace (32
namespaces). A validated protocol flag or sink observation is recorded before
balance/config/code awaits. Only a newer fully admitted explicit false role can
clear it; cancellation, provider/DB failures and reserve copies cannot. Factory inventory cannot clear the separate tax-ledger registry
quarantine; that remains unavailable until an authoritative token-context recovery
protocol exists. In-flight cache, wrapper and deeper result publication rechecks
the current epoch/generation under the same guard lock. Actor context memory remains bounded
at 2,048 per token and global blacklist contexts at 2,048. Large-factory refresh
coverage is separate from these finite work bounds.

## Criteria and verification instructions

A checked result in [RESULTS.md](RESULTS.md) means the complete stated scope,
not production deployment. Review source assertions as well as command status.

| Criterion | Instructions for the reviewing agent |
| --- | --- |
| C1: direct-wrapper floors | Run the wrapper-only frontend suites and local wrapper/mode tests. Verify scheduled capability admission, missing/unknown/expired capability refusal, both directions, target floors, generation invalidation, exact protected message variants and zero preparation queries. In the companion external repo, execute wrap/unwrap rollback tests against actual mint/send replies and compare recipient balance deltas; repeat failures after correction. |
| C2: stateful tax parity or explicit unsupported | Inspect supported-token admission: nonzero cooldown and enabled AutoLP stay unavailable because the public protocol lacks last-trade and AutoLP settlement state. Run unsupported-state regressions with manager exemption too. Do not treat unchanged unsupported behavior as modeled execution or a verification skip. |
| C3: uncensused tax wallets | Exercise configured zero-balance wallets and strictly ordered holder pages outside `traders`; rotate under the documented budget. Require fresh factory blacklist and per-token flags/balances, enforce max-wallet, poison malformed pages/provider failures, expire the original observation and retain known negatives/capacity quarantine. Disable providers before quotes and count zero request-stage calls. |
| C4: composed inverse parity | Reproduce the formerly rejected two-hop rounding dip and first-hop dips. Compare supported inverses with independent brute-force integer arithmetic, including taxes, fees and mixed reserves; prove target sufficiency and first-reaching input. Exercise budget exhaustion and preserve healthy alternatives. |

## Actual wrapper chain verification

The reusable [LocalTerra driver](../../../scripts/qa/localterra-wrapper-floors-1459.py)
creates fresh synthetic contracts/keys without resetting shared volumes. Build
the companion treasury and mapper artifacts with a supported CosmWasm toolchain,
then run twice (use distinct receipt/env paths):

```bash
python3 scripts/qa/localterra-wrapper-floors-1459.py \
  --container YOUR_LOCALTERRA_CONTAINER \
  --artifacts /absolute/path/to/companion/artifacts \
  --cw20-artifact smartcontracts/artifacts/cw20_mintable.wasm \
  --receipt /tmp/wrapper-floor-receipt.json \
  --env-file /tmp/wrapper-floor-public.env
```

Its exact wrong-floor errors, custody/supply/rate snapshots and distinct recipient
prove actual bank-net delivery and atomic rollback; ante fees remain payable on
failed transactions. Raw key-generation output is never public evidence. The
public stack env can be supplied to `VERIFY_ISSUE_1381_WRAP_STACK_ENV` for existing
native/taxed-route flow verification. Artifacts and recorded hashes are synthetic
test inputs; this does not replace the release provenance under #1456.

## Agent procedure and skip policy

1. Create an isolated worktree. Use the repository Node wrapper and install
   dependencies. Database binaries use private `indexer/.env.qa` and create
   per-process disposable databases under [DB1396](../../../skills/AGENTS_INDEXER_DATABASE_ISOLATION.md).
   Never migrate or seed a shared application database.
2. Read the four criterion rows and linked invariant files. Run
   `make verify-issue-1452`, `make verify-issue-1398`,
   `make verify-issue-1381` and `make verify-issue-1390`.
   For funded receive/provider budget boundaries also run `make verify-issue-1422`;
   database fixture changes follow `make verify-issue-1396`. The source verifiers repeat their meaningful suites; use a stable source
   fingerprint. Include the companion external protocol tests before claiming C1.
3. Inspect actual payloads, callback rollback snapshots, input/output integers,
   budgets, original observation times, generations and provider counters.
   Correct executed failures and rerun affected checks; run complete integration
   verification after the final source edit.
4. Record revision, UTC timestamps, exact commands, counts, distributions/maxima
   and missing capabilities in RESULTS. Check off only verified criterion scope.
5. If unable to complete a check, record **SKIP**, the attempted command and
   concrete unavailable prerequisite; leave that acceptance item unchecked.
   Overall **PASS WITH SKIPS** is permitted when every executed check passes.
   Unavailable holds/gates do not block this source PR. Failed assertions remain
   **FAIL**, and runtime controls or unimplemented semantics cannot be waived.

Production deployment, loaded/reference-network/device acceptance, historical
reconstruction and active providers remain on [#1456](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1456).
Skipped rollout does not establish deployed compatible wrapper messages. Old
contracts continue to reject protected variants; observations from old contracts
keep direct signing disabled. DeFiLlama scope remains on
[#1449](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1449).
