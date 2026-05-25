# Agent playbook: Terra Classic block explorer URLs

Use when adding or changing **“View on explorer”** links for **transactions** or **bech32 addresses**, or when touching [`terraExplorer.ts`](../frontend-dapp/src/utils/terraExplorer.ts).

## Canonical references

| Doc / issue | Purpose |
|-------------|---------|
| [docs/frontend.md § Terra Classic block explorer URLs](../docs/frontend.md#terra-classic-block-explorer-urls) | Invariants, URL matrix per `VITE_NETWORK` |
| [GitLab #184](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184) | `getExplorerAddressUrl` |
| [`chainlist.json`](../frontend-dapp/public/chains/chainlist.json) | `explorerUrl` for `columbus-5` / `rebel-2` |
| [`terraExplorer.test.ts`](../frontend-dapp/src/utils/__tests__/terraExplorer.test.ts) | Vitest per network branch |

## Rules of thumb

1. **Import helpers, not hosts:** `getExplorerTxUrl(hash)` and `getExplorerAddressUrl(addr)` from [`terraExplorer.ts`](../frontend-dapp/src/utils/terraExplorer.ts).
2. **Network follows the build:** `VITE_NETWORK` → `DEFAULT_NETWORK` (`local` \| `mainnet` \| `testnet`). Do not infer network from the address string.
3. **Local = LCD REST:** tx → `/cosmos/tx/v1beta1/txs/…`; address → `/cosmos/auth/v1beta1/accounts/…` on `NETWORKS.local.terra.lcd`.
4. **Public = Finder paths:** `{explorerUrl}/tx/…` and `{explorerUrl}/address/…` (trailing slashes normalized on the base).
5. **Hide when null:** If the helper returns `null`, omit the anchor (no dead links).

## Cross-links

- Address row (address explorer consumer): [`AGENTS_FRONTEND_ADDRESS_ROW.md`](./AGENTS_FRONTEND_ADDRESS_ROW.md) · [#188](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/188)
- Wallet chip dropdown (consumer): [`AGENTS_FRONTEND_WALLET_CHIP.md`](./AGENTS_FRONTEND_WALLET_CHIP.md) · [#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185) — [docs § Connected wallet dropdown](../docs/frontend.md#connected-wallet-dropdown)
- Order history tx column: [`AGENTS_FRONTEND_ORDER_HISTORY.md`](./AGENTS_FRONTEND_ORDER_HISTORY.md) (`getExplorerTxUrl` only)
