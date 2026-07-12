# Agent playbook: Terra Classic block explorer URLs

Use when adding or changing **“View on explorer”** links for **transactions** or **bech32 addresses**, or when touching [`terraExplorer.ts`](../frontend-dapp/src/utils/terraExplorer.ts) / columbus-5 `explorerUrl` in [`chainlist.json`](../frontend-dapp/public/chains/chainlist.json).

## Canonical references

| Doc / issue | Purpose |
|-------------|---------|
| [docs/frontend.md § Terra Classic block explorer URLs](../docs/frontend.md#terra-classic-block-explorer-urls) | Invariants, URL matrix per `VITE_NETWORK` |
| [GitLab #184](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184) | `getExplorerAddressUrl` |
| [GitLab #478](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/478) | Galaxy Finder mainnet path is `/columbus-5`, not `/mainnet` |
| [GitLab #430](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/430) | SEC-E10 segment validation (keep intact) |
| [`chainlist.json`](../frontend-dapp/public/chains/chainlist.json) | `explorerUrl` for `columbus-5` / `rebel-2` |
| [`terraExplorer.test.ts`](../frontend-dapp/src/utils/__tests__/terraExplorer.test.ts) | Vitest per network branch |

## Rules of thumb

1. **Import helpers, not hosts:** `getExplorerTxUrl(hash)` and `getExplorerAddressUrl(addr)` from [`terraExplorer.ts`](../frontend-dapp/src/utils/terraExplorer.ts).
2. **Network follows the build:** `VITE_NETWORK` → `DEFAULT_NETWORK` (`local` \| `mainnet` \| `testnet`). Do not infer network from the address string.
3. **Local = LCD REST:** tx → `/cosmos/tx/v1beta1/txs/…`; address → `/cosmos/auth/v1beta1/accounts/…` on `NETWORKS.local.terra.lcd`.
4. **Public = Finder paths:** `{explorerUrl}/tx/…` and `{explorerUrl}/address/…` (trailing slashes normalized on the base).
5. **Mainnet Galaxy Finder = chain-id segment ([#478](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/478)):** `columbus-5` `explorerUrl` must be `https://finder.terraclassic.community/columbus-5`. Do **not** use `/mainnet` — that path does not resolve txs/addresses reliably on Galaxy Finder. Testnet Hexxagon (`rebel-2` → `/testnet`) is a separate product; leave it alone unless verified.
6. **Hide when null:** If the helper returns `null`, omit the anchor (no dead links).
7. **Validate segments ([#430](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/430)):** Tx hash = 64 hex digits; address = valid `terra` bech32. Reject `javascript:`, HTML specials, empty strings — return `null` so components never render an injectable `href`. Adversarial cases in [`terraExplorer.test.ts`](../frontend-dapp/src/utils/__tests__/terraExplorer.test.ts); link rendering in [`AddressRow.explorerSafety.test.tsx`](../frontend-dapp/src/components/ui/__tests__/AddressRow.explorerSafety.test.tsx) and [`TerraBroadcastPendingLink.test.tsx`](../frontend-dapp/src/components/ui/__tests__/TerraBroadcastPendingLink.test.tsx).

## Do not

- Hardcode `finder.terraclassic.community` (or any Finder host) in React components.
- “Fix” mainnet links by switching to Hexxagon or another host without maintainer approval.
- Weaken SEC-E10 adversarial tests when changing `explorerUrl`.
- Change `rebel-2` / Hexxagon bases as part of a columbus-5 path fix.

## Cross-links

- Address row (address explorer consumer): [`AGENTS_FRONTEND_ADDRESS_ROW.md`](./AGENTS_FRONTEND_ADDRESS_ROW.md) · [#188](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/188)
- Wallet chip dropdown (consumer): [`AGENTS_FRONTEND_WALLET_CHIP.md`](./AGENTS_FRONTEND_WALLET_CHIP.md) · [#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185) — [docs § Connected wallet dropdown](../docs/frontend.md#connected-wallet-dropdown)
- Order history tx column: [`AGENTS_FRONTEND_ORDER_HISTORY.md`](./AGENTS_FRONTEND_ORDER_HISTORY.md) (`getExplorerTxUrl` only)
