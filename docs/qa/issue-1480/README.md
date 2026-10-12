# Offline execution-budget verification

Issue [#1480](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1480).
Implementation PR: recorded in [results](RESULTS.md).
Read [OB1480-1–OB1480-10](../../../skills/AGENTS_OFFLINE_SWAP_BUDGET.md),
[auto/offline gas policy](../../adr/0014-swap-auto-gas.md),
[local quotes](../../../skills/AGENTS_LOCAL_SWAP_QUOTES.md),
[net floors](../../../skills/AGENTS_NET_EXECUTION_FLOORS.md) and
[route-local tax](../../../skills/AGENTS_ROUTE_LOCAL_TAX.md).

## Agent procedure

1. Record exact revision/source fingerprint and UTC. Read issue/comments through
   openbao-tools `bao-exec` + `fj`. Synthetic actors belong only to owned fixtures.
2. Run `make verify-issue-1480`. Run `1360`, `1328`, `1264`, `1222`, `1452`, `1473`,
   `1381`, `1398`, `1390` checks. Keep legacy and funding-v1 execution separate.
3. Run the owned funded gas fixture with checksum/source-verified current artifacts.
   Record exact requested gas, included code/gas used and independently computed
   delivered net amount. Legacy static-budget failures are evidence, not PASS.
4. Measure supported wrap/unwrap/tax/hook/book families separately. A pool-only
   measurement does not calibrate arbitrary callback gas or certify every wallet.
5. Fix failures, recreate only owned fixture state, and repeat on stable final
   source. Inspect actual entry builders and broadcast boundaries, fee rows,
   Max/native pay+fee reserve, wallet rewrites, >5%/>30%/≥99% and no-resend paths.
6. Attest deployed frontend/backend and contract artifacts separately, run public
   API/browser regressions and bounded [monitoring](../issue-1479/README.md#monitoring).

## Full issue acceptance

A whole criterion is complete only when all body/comment requirements pass.
Detailed passing subsets, skipped scope and failures belong in [results](RESULTS.md).

| Complete | Criterion | Required evidence |
| --- | --- | --- |
| [ ] | AC1 classify reported families | Funded LUNC/USTC/CL8Y→USTR evidence distinguishing underbudget, wallet rewrite, legacy/provider or genuine protection; retain unavailable original-report details. |
| [ ] | AC2 funded settlement | Small/typical supported native/CW20, legacy/v1 and non-USTR controls code0, gas_used<gas_wanted, exact net/floors; unsupported/over-limit blocks before signing. |
| [ ] | AC3 one fee budget | Actual Swap/Trade fee rows, Max and native reserve match requested/signed budget; native/CW20 low balance/rounding boundaries and wallet limitations. |
| [ ] | AC4 zero quote RPC | Provider counters stay zero throughout quote/refresh/confirmation/preparation; no input-driven calibration; retain generation/expiry. |
| [ ] | AC5 distinct protections | Legitimate impact confirmations and 30%/99% rules, compatible references and consistent percent; no gas/protection remedy confusion. |
| [ ] | AC6 no resend | Included code11/unknown inclusion, rejection/disconnect, sequence and concurrent submits retain one intentional flow. |
| [ ] | AC7 docs/retest/deployment | Current offline contract, exact-revision source and funded receipts, independent deployed frontend/backend/contract/wallet provenance. |

## Funded fixture

`scripts/qa/localterra-offline-gas-1480.py` accepts an explicitly owned container
and artifact directory. The script checks tracked checksums and source-input
hashes, separates the synthetic treasury/recipient, computes raw outputs from
read reserves and captures included gas. Artifact receipts describe any rebuild
provenance limits explicitly. No production signing/reset occurs.

## Explicit skip policy

Complete runnable checks first. Unavailable checks may be explicit SKIP with
attempted command and concrete reason, and **PASS WITH SKIPS** only if every
executed check passes. Executed failures remain FAIL. A source helper test, a quote
HTTP200 or one synthetic pool family does not complete reported-wallet, wrapper,
callback or production acceptance. Keep corresponding criteria unchecked and the
issue open until all body/comment criteria pass.
