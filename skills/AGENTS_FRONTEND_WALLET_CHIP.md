# Agent playbook: connected wallet chip (header)

Use when changing **`WalletButton.tsx`**, **`WalletDropdownMenuItems.tsx`**, **`WalletLuncBalance.tsx`**, connected-state dropdown affordances, or header wallet styling in **`index.css`**.

## Canonical references

| Doc / issue | Purpose |
|-------------|---------|
| [docs/frontend.md § Connected wallet chip — LUNC balance](../docs/frontend.md#connected-wallet-chip-lunc-balance) | Invariants: bank `uluna`, cache key, chip vs menu placement |
| [docs/frontend.md § Connected wallet dropdown](../docs/frontend.md#connected-wallet-dropdown) | Copy / explorer / switch wallet menu rows ([#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185)) |
| [GitLab #140](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/140) | Balance / copy / explorer / switch wallet (scope **B**) |
| `frontend-dapp/src/hooks/useNativeUlunaBalance.ts` | Shared `['tokenBalance', addr, 'uluna']` query with swap/pool/trade gas gates |
| `frontend-dapp/src/components/wallet/WalletLuncBalance.tsx` | Formatted LUNC line for chip + menu header |
| `frontend-dapp/src/components/wallet/WalletDropdownMenuItems.tsx` | Labeled menu rows: Copy address, View on explorer, Switch wallet |

## Rules of thumb

1. **LUNC only in the chip today:** native gas is always **bank `uluna`** (6 decimals, symbol **LUNC**). Do not show CW20 “LUNC-C” here — that is escrow on limit/swap forms.
2. **Reuse `useNativeUlunaBalance`:** do not add a parallel LCD query; invalidate via existing `['tokenBalance']` prefixes after txs.
3. **Visible without opening the menu:** desktop (`sm+`) and mobile chip triggers must include **`WalletLuncBalance`**; the dropdown header repeats balance + **`AddressRow`** (full bech32 + inline copy/explorer).
4. **Menu dismiss + Escape ([#187](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/187)):** when open, render **`app-menu-dismiss`** and register **`Escape`** on `window` only while `showDropdown` is true.
5. **Dropdown menu rows ([#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185)):**
   - **Copy address** — `CopyButton` with `menuLabel="Copy address"`.
   - **View on explorer** — `getExplorerAddressUrl`; hide when `null`.
   - **Switch wallet** — `disconnect()` then `setWalletModalOpen(true)`; portal `WalletModal` while connected.
   - Keep **Trader profile** and **Disconnect** in `WalletButton`.
6. **Siblings still open:** [#186](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186) chip network/mobile layout polish.

## Cross-links

- Address row (header): [`AGENTS_FRONTEND_ADDRESS_ROW.md`](./AGENTS_FRONTEND_ADDRESS_ROW.md) ([GitLab **#188**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/188))
- Clipboard primitive: [`AGENTS_FRONTEND_COPY_BUTTON.md`](./AGENTS_FRONTEND_COPY_BUTTON.md) ([GitLab **#183**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/183))
- Explorer URL helpers: [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md) ([GitLab **#184**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184))
- Connect modal: [`AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md`](./AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md)
- Responsive header shell: [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md)
- Native gas gates (same balance hook): [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)
- Keyboard focus: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
