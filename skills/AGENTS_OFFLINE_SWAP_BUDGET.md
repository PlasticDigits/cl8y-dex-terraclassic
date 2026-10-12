# Reviewed offline swap budgets (#1480)

Apply when changing local Swap/Trade fees, Max, native balance reservation,
message preparation, gas-family classification or signing. Issue:
[#1480](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1480).
Read [the agent checklist](../docs/qa/issue-1480/README.md),
[gas policy](../docs/adr/0014-swap-auto-gas.md),
[LQ1452](AGENTS_LOCAL_SWAP_QUOTES.md), [NE1398](AGENTS_NET_EXECUTION_FLOORS.md)
and [RT1381](AGENTS_ROUTE_LOCAL_TAX.md).

| ID | Invariant |
| --- | --- |
| OB1480-1 | Reviewed local paths remain offline. Quote/refresh/review/preparation never invokes LCD gas simulation or independently scheduled calibration. Non-local historical auto-gas is a separate policy. |
| OB1480-2 | Select a generic budget from the exact serialized execution family, operations, modes and funding version. Symbols/asset exceptions or unverified route metadata never choose cheaper gas. |
| OB1480-3 | Display, Max/native reserve and requested Fee share the same checked plan budget. Positive hybrid work retains its actual book/maker caps; one reviewed router hop remains a router execute. |
| OB1480-4 | Bound final total gas including all messages and wrap combination costs. Unknown/unsupported/malformed/over-limit offline families fail before wallet authorization/signing, never falling through to BASE_GAS_LIMIT. |
| OB1480-5 | Fee denom remains uluna. Gas price minima, positive safe integer budgets, upward fee rounding and native pay plus fee reservation remain checked; invalid/overflow fees are not displayed as ready. |
| OB1480-6 | Measure requested/signed gas separately from gas price/fee amount. Keep existing wallet guard scope and preferNoSetFee; atomic APIs that cannot inspect signed fields remain explicitly unverified. |
| OB1480-7 | Preserve principal/floors, original quote expiry and generation/context binding. Higher gas does not authorize extra principal or replace reviewed operations. |
| OB1480-8 | Price protection and gas failures remain distinct. Genuine >5% confirmation, >30% Expert and ≥99% protection remain. Missing/malformed references never mean zero impact. |
| OB1480-9 | Included code 11 and unknown inclusion never trigger automatic resend. Preserve sequence mismatch recovery and one intentional submit flow. |
| OB1480-10 | Run `make verify-issue-1480` and applicable existing verifiers first/retest. Calibration needs funded current-artifact evidence, independently derived net balances and rollback, not constants mirrored in assertions. Skips remain unchecked; source and synthetic settlement cannot attest physical wallets or deployed contract artifacts. |
