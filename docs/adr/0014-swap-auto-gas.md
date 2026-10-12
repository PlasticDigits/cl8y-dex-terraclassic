# ADR 0014: Swap auto gas at 1.2× simulated use

## Status

Accepted for the remaining non-local execute path ([#1360](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1360)). Reviewed local Swap/Trade routes use the offline contract below; the historical auto-gas policy does not apply to them. Physical wallet acceptance remains separate.

## Current local/offline boundary (#1480)

[#1452](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1452) prohibits user-triggered simulation during quote, refresh, preflight and reviewed-route preparation. [#1480](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1480) therefore calibrates generic offline route families from owned funded current-artifact executions rather than enabling this ADR's LCD helper in Swap/Trade.

The [offline classifier](../../frontend-dapp/src/services/terraclassic/offlineSwapBudget.ts) uses **800,000 + 2,400,000 × hops** for legacy router core settlement and **800,000 + 1,800,000 × hops** for funding-v1. Each exceeds 1.2 times the recorded plain-CW20 empty-book maxima for one through four hops. Existing bounded hybrid work, wrap/unwrap settlement and combination overhead remain additive; the complete transaction must fit the **15,000,000** client policy ceiling. That ceiling is not chain-limit attestation. Unknown message variants, funding versions, malformed/oversized hooks, excessive hops/maker caps and over-limit combinations are unavailable for signing, never a BASE_GAS_LIMIT fallback. Calibration runs independently in owned fixtures; UI input cannot schedule it.

The actual Swap/Trade fee row, native balance gate, native Max and canonical offline requested Fee use this shared classifier with the reviewed operation and funding fields. Display rounding does not change integer uluna reservation. One local legacy router hop is a router execute, not the direct-pair allowance sequence. Hybrid previews use the real split/cap instead of an invented zero-book leg. Reviewed legacy metadata cannot inherit a separately cached funding-v1 version.

On split wallet APIs, [signed Fee inspection](../../frontend-dapp/src/services/terraclassic/signedOfflineSwapFee.ts) requires the exact reviewed gas, uluna amount, payer and granter before broadcasting. Both lower and higher wallet rewrites require renewed review. This check applies only to offline reviewed routes; historical global extension guard scope and `preferNoSetFee` remain intact. Atomic Station/LuncDash WalletConnect post exposes no signed bytes before inclusion: requested Fee parity is tested, signed/device behavior remains explicit acceptance work.

The funded generic fixtures do not attest named production USTR routes, arbitrary hooks, community-tax implementations, wrapper dependencies or deployed frontend/backend artifacts. Preserve positive per-hop/post-hook/final floors, gross funding and original reviewed expiry; higher gas grants no additional principal or economic permission. Included code 11 remains terminal and is never automatically resent. Independent [reference-policy exclusion](../../frontend-dapp/src/utils/routeQuoteErrors.ts) is a pricing outcome, not a gas error; it cannot become a zero-impact approval.

Verification and sanitized funded receipts: [issue #1480 checklist](../qa/issue-1480/README.md). The historical invariants below describe the non-local auto-gas policy; helper-only fee-hint tests do not establish current production entry coverage.

## Context

Retail CL8Y → KENA swaps signed a static two-hop `gas_wanted` of **1,910,000** and were included with `code` 11 (`out of gas`) at about **1,937,97x** gas used. The same limit failed for ~0.54 CL8Y and for 0.05 CL8Y. Each inclusion still charged the fee. A later LUNC wrap plus one cLUNC → KENA hop fit inside its own static envelope; that is a different message shape.

[#1328](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1328) / [ADR 0013](./0013-cl8y-ust1-gas-floor.md) had added a pair-specific **3,000,000** floor for CL8Y → UST1 only. Extending that pattern with a KENA constant would leave the next pair on the same static number.

`SWAP_GAS_BUFFER` (**1.3**) scales the static per-hop formula. It is not this ticket’s multiplier.

## Decision

For swap-shaped executes (`swap`, `execute_swap_operations`, `wrap_deposit`, and CW20 `send` hooks that wrap those messages):

1. Build the unsigned transaction that will be signed.
2. `POST /cosmos/tx/v1beta1/simulate` once, with an 8s timeout.
3. If `gas_used` is an integer in **[100,000, 15,000,000]** (`HYBRID_SWAP_GAS_LIMIT`), sign `Fee.gas = ceil(gas_used × 1.2)` and `Fee.amount` in `uluna` at `effectiveGasPriceUluna()`.
4. Otherwise sign the existing global per-hop / per-message envelope (`getGasLimitForTx` / `totalGasLimitForExecuteMsgs`).

The gas limit inside the simulate request exists so the query can finish. It is not copied into the signed fee.

`cl8yUst1PoolOnlyTwoHopGasLimit` is not part of this decision. No KENA or other pair-address constant is added. A failed two-hop simulate, including CL8Y → UST1 and CL8Y → KENA, signs **1,910,000**. A failed wrap + one hop signs **1,800,000**.

The Network fee row uses the same rule: the 1.2× amount after a successful simulate, otherwise the static uluna amount. Preview messages can omit the submit-time deadline, so the displayed uluna can differ in the last digits from the signed fee. The signed fee is always one fresh simulate of the exact messages, unless an identical successful simulate is still in the 15s cache.

The first simulate uses the 15,000,000 query ceiling. If it does not finish, the query gas starts at the static envelope plus **200,000** and climbs by **200,000** until a simulate returns a usable `gas_used` or three replies contain no `gas_used`. That climb happens before the signature. An included `code` 11 does not broadcast again. `max_spread` and `minimum_receive` are unchanged. Keplr and Station extension keep `preferNoSetFee`. Station WalletConnect atomic post remains the [ADR 0004](./0004-terraclassic-retail-gas-census.md) **G-AUTO-8** residual.

While broadcast phase is `recovering` or `confirming`, the pay-field control **Use {amount} instead** is hidden. When route slippage is above 5%, the percent outside collapsed trade details is that route figure, not a different worst-hop percent.

The out-of-gas sentence for `gas_used > gas_wanted + 1,000` is the constant `OUT_OF_GAS_SHORT_ESTIMATE_MESSAGE`. It does not interpolate the raw log.

## Consequences

If simulate fails, two-hop CL8Y routes can still sign **1,910,000**, which the measured uses exceed. Closing that residual with a pair floor is out of scope. A later change may raise the single global per-hop constant for every pair.

A `gas_used` of 1 is rejected (below 100,000) so a dummy body cannot sign a 2-gas fee. A `gas_used` of 147,000,000 (the Station mobile figure in [#679](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/679)) falls back to the per-hop fee instead of `1.2 × 147e6`.

## Invariants G1360-1–G1360-8

| ID | Rule |
|----|------|
| **G1360-1** | Swap `Fee.gas` is `ceil(gas_used × 1.2)` from one LCD simulate of the signed messages. Denom stays `uluna`. |
| **G1360-2** | Timeout, non-success HTTP, missing or non-integer `gas_used`, `gas_used` below **100,000**, or `gas_used` above **15,000,000** selects the static envelope. Do not multiply an over-cap result. |
| **G1360-3** | No token-address gas constant. Failed two-hop sim is **1,910,000**. Failed wrap+1hop is **1,800,000**. Successful sim is not raised to **3,000,000**. |
| **G1360-4** | Swap and Trade market Network fee show that resolved uluna amount. |
| **G1360-5** | One signature and one broadcast. A failed simulate climbs the query gas by **200,000** until a simulate finishes or three replies have no `gas_used`. Included `code` 11 does not broadcast again. |
| **G1360-6** | High-impact route slippage and the outside percent are the same figure. **Use {amount} instead** is hidden during `recovering` and `confirming` and does not broadcast. |
| **G1360-7** | Out-of-gas copy when used exceeds wanted by more than **1,000** is a fixed sentence with no raw-log interpolation. |
| **G1360-8** | `preferNoSetFee` stays on for Keplr and Station extension. The simulate-request gas limit is not the signed fee. Fixtures do not embed a trader address. |

Playbook: [`skills/AGENTS_SWAP_AUTO_GAS.md`](../../skills/AGENTS_SWAP_AUTO_GAS.md). Verify: `make verify-issue-1360`.
