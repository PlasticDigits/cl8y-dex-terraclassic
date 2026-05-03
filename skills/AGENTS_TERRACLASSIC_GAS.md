# Agent playbook: Terra Classic swap gas (frontend)

Use this when debugging **`out of gas`** on swap txs, tuning gas constants, or reviewing PRs that touch fee/gas code.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/contracts-terraclassic.md § Factory storage & upgrades](../docs/contracts-terraclassic.md#factory-storage--upgrades) | Pair-address registry map (`pair_addr_reg`), migrate **1.0.0 → 1.1.0**, iteration only for pagination / broadcast-all ([GitLab #122](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/122)) |
| [docs/frontend.md § Terra Classic gas limits](../docs/frontend.md#terra-classic-gas-limits) | Invariants, formula, regression anchor ([GitLab #115](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/115)) |
| [docs/frontend.md § Production build — source maps](../docs/frontend.md#vite-production-sourcemaps) · [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md) | Vite prod must not ship public source maps ([GitLab #117](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/117)) — separate from gas tuning |
| [docs/frontend.md § Simulated (dev) wallet / bundle secrets](../docs/frontend.md#simulated-dev-wallet-and-vite_dev_mnemonic) · [`AGENTS_BUNDLE_DEV_WALLET.md`](./AGENTS_BUNDLE_DEV_WALLET.md) | No mnemonic literals in `src/`; `VITE_DEV_MNEMONIC` only in dev env ([GitLab #118](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/118)) |
| `frontend-dapp/src/utils/constants.ts` | `SWAP_GAS_PER_HOP`, `SWAP_GAS_BUFFER`, floors, padding |
| `frontend-dapp/src/services/terraclassic/transactions.ts` | `getGasLimitForTx`, hybrid vs pool-only branches |
| `frontend-dapp/src/services/terraclassic/__tests__/transactions.test.ts` | Expected `Fee.gasLimit` values — **update when constants change** |

## Rules of thumb

1. **Do not assume LCD simulation** matches what Station broadcasts; the dApp supplies an explicit `Fee` object.
2. **If `gasUsed` from a failed tx is known**, new defaults must exceed that with margin (wasm + storage variance on Classic is not tight).
3. **Keep CLI and dApp aligned**: repo scripts use `--gas-adjustment 1.3` on `terrad`; frontend `SWAP_GAS_BUFFER` should not drift far below that culture without justification.
4. **Changing `SWAP_GAS_BUFFER`** scales all hop counts; re-run unit tests and, when possible, a LocalTerra swap happy path.
5. **Factory governance gas ([GitLab #122](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/122)):** Per-pair admin messages (`SetPairFee`, `SetPairHooks`, `SweepPair`, …) must use **O(1)** on-chain lookups (`pair_addr_reg`), not linear scans over all pairs. Full-registry iteration remains only for intentional broadcast/pagination paths; see [`docs/contracts-terraclassic.md` § Factory storage & upgrades](../docs/contracts-terraclassic.md#factory-storage--upgrades).

## When the user references another repo for “how gas works”

Paths differ by machine. Compare **patterns** (simulate vs fixed gas, adjustment multipliers), not copy-paste numbers, unless the same contracts and message shapes are guaranteed.
