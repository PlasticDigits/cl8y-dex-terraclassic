# Agent playbook: connected wallet chip (header)

Use when changing **`WalletButton.tsx`**, **`WalletDropdownMenuItems.tsx`**, **`WalletChipNetworkIndicator.tsx`**, **`WalletLuncBalance.tsx`**, connected-state dropdown affordances, or header wallet styling in **`index.css`**.

## Canonical references

| Doc / issue | Purpose |
|-------------|---------|
| [docs/frontend.md § Connected wallet chip — LUNC balance](../docs/frontend.md#connected-wallet-chip-lunc-balance) | Invariants: bank `uluna`, cache key, chip vs menu placement |
| [docs/frontend.md § Connected wallet chip — network & mobile](../docs/frontend.md#connected-wallet-chip-network-mobile) | `shortLabel` on `sm+` trigger, mobile chip vs menu strategy ([#186](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186)) |
| [docs/frontend.md § Connected wallet dropdown](../docs/frontend.md#connected-wallet-dropdown) | Copy / explorer / switch wallet menu rows ([#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185)) |
| [GitLab #140](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/140) | Balance / copy / explorer / switch wallet (scope **B**) |
| `frontend-dapp/src/hooks/useNativeUlunaBalance.ts` | Shared `['tokenBalance', addr, 'uluna']` query with swap/pool/trade gas gates |
| `frontend-dapp/src/components/wallet/WalletLuncBalance.tsx` | Formatted LUNC line for chip + menu header |
| `frontend-dapp/src/components/wallet/WalletDropdownMenuItems.tsx` | Labeled menu rows: Copy address, View on explorer, Switch wallet |

## Rules of thumb

1. **LUNC only in the chip today:** native gas is always **bank `uluna`** (6 decimals, symbol **LUNC**). Do not show CW20 “LUNC-C” here — that is escrow on limit/swap forms.
2. **Reuse `useNativeUlunaBalance`:** do not add a parallel LCD query; invalidate via existing `['tokenBalance']` prefixes after txs.
3. **Visible without opening the menu:** desktop (`sm+`) and mobile chip triggers must include **`WalletLuncBalance`**; the dropdown header repeats balance + **`AddressRow`** (full bech32 + inline copy/explorer).
4. **Network on trigger ([#186](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186) — done):** use [`WalletChipNetworkIndicator.tsx`](../frontend-dapp/src/components/wallet/WalletChipNetworkIndicator.tsx) + `getNetworkBadgeCopy()` — logo at all widths, **`shortLabel` text from `sm:`** only. Mobile: balance + truncated address on chip; labeled copy/explorer/switch rows stay in the menu ([#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185)). `data-testid="wallet-network-short-label"`.
5. **Menu dismiss + Escape ([#187](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/187) — done):** when open, render **`app-menu-dismiss`** and register **`Escape`** on `window` only while `showDropdown` is true.
6. **Dropdown above EnvironmentRibbon ([#486](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/486)):** `.wallet-menu` shares sticky-stack stacking with More — keep `.app-header-shell` above `.app-env-ribbon` inside `.app-top-sticky`. See [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md).
7. **Dropdown menu rows ([#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185) — done):**
   - **Copy address** — `CopyButton` with `menuLabel="Copy address"`.
   - **View on explorer** — `getExplorerAddressUrl`; hide when `null`.
   - **Switch wallet** — `disconnect()` then `setWalletModalOpen(true)`; portal `WalletModal` while connected.
   - Keep **Trader profile** and **Disconnect** in `WalletButton`.

## Cross-links

- Address row (header): [`AGENTS_FRONTEND_ADDRESS_ROW.md`](./AGENTS_FRONTEND_ADDRESS_ROW.md) ([GitLab **#188**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/188))
- Clipboard primitive: [`AGENTS_FRONTEND_COPY_BUTTON.md`](./AGENTS_FRONTEND_COPY_BUTTON.md) ([GitLab **#183**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/183))
- Explorer URL helpers: [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md) ([GitLab **#184**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184))
- Connect modal: [`AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md`](./AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md)
- Responsive header shell (menu stacking **#486**): [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md)
- Native gas gates (same balance hook): [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)
- Keyboard focus: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
