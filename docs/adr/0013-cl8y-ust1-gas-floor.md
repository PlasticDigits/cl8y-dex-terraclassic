# ADR 0013: CL8Y → UST1 pool-only router gas floor

## Status

Proposed. The measured route-specific floor is implemented in this change. A signed Columbus-5 swap and toast check remain operator QA.

## Context

Forgejo [#1328](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1328) reports an out-of-gas failure for retail CL8Y → UST1 Swap. The dApp signs a static `Fee.gas` envelope from `terraGas.ts`; it does not simulate every execute before broadcast.

On 2026-09-24, the current Columbus-5 `/route/solve` quote for **1 CL8Y** selected two pool-only router hops, CL8Y → ALPHA → UST1 (`hybrid: null` on both operations). The shared two-hop router envelope was **1,910,000** gas. A read-only LCD simulation of that forward route used **2,643,979–2,643,980** gas. The verifier is [`frontend-dapp/scripts/measureIssue1328Columbus.mjs`](../../frontend-dapp/scripts/measureIssue1328Columbus.mjs); it queries the live route, builds an unsigned `MsgExecuteContract`, uses permissive positive `min_return` / `minimum_receive` values of 1 raw unit, and calls only the LCD simulation endpoint. This measures the execute path and is not a signed transaction `gas_wanted` result.

A successful reverse swap through the same two pool contracts provides an on-chain cross-check: [tx `606577CBC7F24B22AFEA881296D2CAB235F1B2F6941D123FAF71CBFD77F83BC8`](https://finder.terraclassic.community/columbus-5/tx/606577CBC7F24B22AFEA881296D2CAB235F1B2F6941D123FAF71CBFD77F83BC8), height **30,437,489**, was a two-hop pool-only UST1 → ALPHA → CL8Y router send with `gas_wanted=5,000,000` and `gas_used=2,606,968`. This is supporting evidence for the same pair path; it is not the required forward wallet confirmation.

## Decision

Use a **3,000,000 gas** envelope only when the execute payload is a continuous, two-hop pool-only CW20 route whose first offer is the configured CL8Y token and final ask is the configured UST1 token. Match the endpoints and hop continuity rather than pinning the intermediate token or pair address, because route solving can choose a different middle token.

The special case lives inside the shared `getGasLimitForTx` path. Swap's Network fee estimate already uses the same router operations and therefore receives the same limit. The generic per-hop floor, wrap/unwrap envelopes, route selection, and fee denomination remain independent.

## Invariants G1328-1–G1328-7

| ID | Rule |
|----|------|
| **G1328-1** | The 3M floor applies only to exactly two continuous pool-only hops from configured CL8Y to configured UST1. Intermediate token and pair addresses may vary. |
| **G1328-2** | The floor is above the measured forward simulation (**2,643,980**) by at least **356,020** gas. Retune only from another recorded Columbus-5 measurement. |
| **G1328-3** | `hybrid: null` and an explicit `book_input: "0"` are pool-only. The app does not enable a book leg to solve this failure. Hybrid or greedy routes use existing hybrid gas math. |
| **G1328-4** | Quotes cannot lower this floor: the gas helper takes the maximum of the shared route envelope and the route-specific floor. Unknown URL fields such as `gas` or `gas_limit` are ignored. |
| **G1328-5** | Other two-hop routes retain **1,910,000**; USTC → USTR wrap+2hop retains **2,710,000**. The wrap/unwrap combo constants do not change. |
| **G1328-6** | Fee payment stays in `uluna` / LUNC. Neither stablecoin gas nor a change to the user-signed message is introduced. |
| **G1328-7** | LCD simulation is measurement evidence only. Completion still requires one signed CL8Y → UST1 Columbus-5 swap with `gas_used < gas_wanted` and no under-estimate toast. Track that wallet step in the cl8y-pm inbox. |

## Verification

- `make verify-issue-1328` runs the focused gas, inventory, and Network fee tests. Set `VERIFY1328_SIMULATE_ADDRESS` to an address with at least 1 CL8Y to repeat the read-only Columbus-5 simulation in the same command.
- `make measure-issue-1328` runs only the read-only simulation. It does not sign or broadcast.
- Signed wallet confirmation remains an operator step in the card linked from [#1328](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1328).

Implementation: [`terraGas.ts`](../../frontend-dapp/src/services/terraclassic/terraGas.ts), [`constants.ts`](../../frontend-dapp/src/utils/constants.ts), [`terraGas.issue1328.test.ts`](../../frontend-dapp/src/services/terraclassic/__tests__/terraGas.issue1328.test.ts), and the agent playbook [`AGENTS_TERRACLASSIC_GAS.md`](../../skills/AGENTS_TERRACLASSIC_GAS.md).
