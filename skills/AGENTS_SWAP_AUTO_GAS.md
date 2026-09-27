# Agent playbook: swap auto gas (#1360)

Use this when a retail swap is included with `code` 11 (`out of gas`) under a static hop envelope, or when a review wants another pair-specific gas floor.

## What to change

Signed swap gas lives in [`swapAutoGas.ts`](../frontend-dapp/src/services/terraclassic/swapAutoGas.ts) and is applied from [`broadcastTerraExecuteContracts`](../frontend-dapp/src/services/terraclassic/terraBroadcast.ts). The static numbers in [`terraGas.ts`](../frontend-dapp/src/services/terraclassic/terraGas.ts) are the **fallback** only.

Decision record: [ADR 0014](../docs/adr/0014-swap-auto-gas.md). The CL8Y → UST1 **3,000,000** floor in [ADR 0013](../docs/adr/0013-cl8y-ust1-gas-floor.md) is a historical measurement, not a fee input.

## Do this

- Simulate the execute once (`POST /cosmos/tx/v1beta1/simulate`) and sign `ceil(gas_used × AUTO_GAS_ADJUSTMENT)` with `AUTO_GAS_ADJUSTMENT = 1.2`.
- Keep `Fee.amount` in `uluna` via `effectiveGasPriceUluna()`.
- On simulate failure, use `getGasLimitForTx` / `totalGasLimitForExecuteMsgs` (two hops **1,910,000**, wrap+1hop **1,800,000**).
- Cap a usable `gas_used` at `HYBRID_SWAP_GAS_LIMIT` (**15,000,000**). Reject `gas_used` below `AUTO_GAS_MIN_USED` (**100,000**).
- Show that same resolved amount on Swap and Trade market Network fee.
- Hide **Use {amount} instead** while phase is `recovering` or `confirming`.
- When route slippage is above 5%, show that percent outside collapsed details, not a different worst-hop percent.
- Map a deterministic out-of-gas (`gas_used > gas_wanted + 1,000`) to `OUT_OF_GAS_SHORT_ESTIMATE_MESSAGE`.

## Do not do this

- Do not add a KENA, CL8Y, or UST1 address check to pick `Fee.gas`.
- Do not `max()` a successful 1.2× result with `CL8Y_UST1_TWO_HOP_POOL_GAS_LIMIT`.
- Do not copy the simulate-request gas limit into the signed fee.
- Do not broadcast again after an included `code` 11.
- Do not drop `max_spread` or `minimum_receive` inside the gas helper.
- Do not interpolate the raw log into the short-estimate sentence.
- Do not put a trader bech32 in fixtures or docs.
- Do not turn off `preferNoSetFee` for Keplr or Station extension.
- Do not treat Station WalletConnect atomic post as fixed here. That residual stays **G-AUTO-8** in [ADR 0004](../docs/adr/0004-terraclassic-retail-gas-census.md).
- Older notes that say “do not LCD-sim” (G1264-7, #679, #1222) mean: do not replace the static fallback with an uncapped simulate, and do not adopt Station’s ~147M simulate as the fee. #1360 is the capped dApp simulate. Do not delete it to satisfy those lines.

`SWAP_GAS_BUFFER` (**1.3**) still scales only the static formula. It is not `AUTO_GAS_ADJUSTMENT`.

## Invariants G1360-1–G1360-8

| ID | Rule |
|----|------|
| **G1360-1** | One LCD simulate; signed gas is `ceil(gas_used × 1.2)`; denom `uluna`. |
| **G1360-2** | Timeout, bad HTTP, unusable `gas_used`, below 100,000, or above 15,000,000 → static fallback. Never sign `ceil(147_000_000 × 1.2)`. |
| **G1360-3** | Same hop count and same `gas_used` produce the same gas for every CW20 address. Failed two-hop is **1,910,000**. Failed wrap+1hop is **1,800,000**. |
| **G1360-4** | Network fee row matches that resolved amount. |
| **G1360-5** | One simulate and one broadcast per submit. No automatic retry after `code` 11. |
| **G1360-6** | One impact percent. Reduce control hidden during `recovering` / `confirming`. |
| **G1360-7** | Short-estimate out-of-gas copy is a constant. |
| **G1360-8** | `preferNoSetFee` remains. Query gas ≠ signed gas unless the 1.2× math happens to match. |

## Verify

```bash
make verify-issue-1360
make verify-issue-1328
```

Optional read-only Columbus-5 simulate (no broadcast). Set `VERIFY1360_SIMULATE_ADDRESS` to an account that can simulate a CL8Y send:

```bash
make measure-issue-1360
```

A signed wallet check (`gas_used < gas_wanted`, Network fee matches the signed gas) is not part of `make verify-issue-1360`.
