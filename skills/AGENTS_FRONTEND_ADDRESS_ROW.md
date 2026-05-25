# Agent playbook: `AddressRow` (short address + copy + explorer)

Use when surfacing **bech32 / contract addresses** with copy and explorer affordances ([GitLab **#188**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/188), audit umbrella [#140](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/140)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § AddressRow primitive](../docs/frontend.md#addressrow-primitive) | Invariants: shorten defaults, network via build, hide explorer when null |
| [`AddressRow.tsx`](../frontend-dapp/src/components/ui/AddressRow.tsx) | Label + `CopyButton` + explorer icon link |
| [`CopyButton.tsx`](../frontend-dapp/src/components/ui/CopyButton.tsx) | Clipboard ([#183](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/183)) |
| [`terraExplorer.ts`](../frontend-dapp/src/utils/terraExplorer.ts) | `getExplorerAddressUrl` ([#184](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184)) |

## Rules of thumb

1. **Do not** hand-roll shorten + copy + explorer in pages — import **`AddressRow`** (or extend it if a new layout is needed).
2. **Network** follows `VITE_NETWORK` / `DEFAULT_NETWORK` inside `getExplorerAddressUrl`; do not pass per-row network overrides.
3. **`showFull`** for menu / QA surfaces that need the entire bech32 (wallet dropdown); default shortened label elsewhere.
4. **Custom `startChars` / `endChars`** only when design calls for a denser or longer chip (trader header uses 12/6).
5. **Tx hashes** are not addresses — keep `TxResultAlert` / `TradesTable` on `getExplorerTxUrl` + `shortenTxHashForDisplay` until a dedicated row primitive is filed.

## First consumers (MR scope)

| Surface | `data-testid` |
|---------|----------------|
| Wallet dropdown full address | `wallet-menu-address-row` |
| Pool remove LP — LP token line | `pool-lp-token-address-row` |
| Trader profile header | `trader-profile-address-row` |

## Follow-ups (still #188 / #140)

- Pair address chips on Pool / Charts / Limit orders
- `TxResultAlert` tx hash copy (explorer already present)
- Wallet chip trigger shortening stays separate ([#186](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186))

## Cross-links

- Clipboard: [`AGENTS_FRONTEND_COPY_BUTTON.md`](./AGENTS_FRONTEND_COPY_BUTTON.md)
- Explorer URLs: [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md)
- Wallet chip shell: [`AGENTS_FRONTEND_WALLET_CHIP.md`](./AGENTS_FRONTEND_WALLET_CHIP.md)
- Keyboard focus on icon controls: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
