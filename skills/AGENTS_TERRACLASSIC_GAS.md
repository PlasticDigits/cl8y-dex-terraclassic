# Agent playbook: Terra Classic swap gas (frontend)

Use this when debugging **`out of gas`** on swap txs, tuning gas constants, or reviewing PRs that touch fee/gas code.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Terra Classic gas limits](../docs/frontend.md#terra-classic-gas-limits) | Invariants, formula, regression anchor ([GitLab #115](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/115)) |
| [docs/frontend.md § Production build — source maps](../docs/frontend.md#vite-production-sourcemaps) · [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md) | Vite prod must not ship public source maps ([GitLab #117](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/117)) — separate from gas tuning |
| `frontend-dapp/src/utils/constants.ts` | `SWAP_GAS_PER_HOP`, `SWAP_GAS_BUFFER`, floors, padding |
| `frontend-dapp/src/services/terraclassic/transactions.ts` | `getGasLimitForTx`, hybrid vs pool-only branches |
| `frontend-dapp/src/services/terraclassic/__tests__/transactions.test.ts` | Expected `Fee.gasLimit` values — **update when constants change** |

## Rules of thumb

1. **Do not assume LCD simulation** matches what Station broadcasts; the dApp supplies an explicit `Fee` object.
2. **If `gasUsed` from a failed tx is known**, new defaults must exceed that with margin (wasm + storage variance on Classic is not tight).
3. **Keep CLI and dApp aligned**: repo scripts use `--gas-adjustment 1.3` on `terrad`; frontend `SWAP_GAS_BUFFER` should not drift far below that culture without justification.
4. **Changing `SWAP_GAS_BUFFER`** scales all hop counts; re-run unit tests and, when possible, a LocalTerra swap happy path.

## When the user references another repo for “how gas works”

Paths differ by machine. Compare **patterns** (simulate vs fixed gas, adjustment multipliers), not copy-paste numbers, unless the same contracts and message shapes are guaranteed.
