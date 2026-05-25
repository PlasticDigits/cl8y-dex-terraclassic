# Agent playbook: connected wallet chip (header)

Use when changing **`WalletButton.tsx`**, **`WalletLuncBalance.tsx`**, connected-state dropdown affordances, or header wallet styling in **`index.css`**.

## Canonical references

| Doc / issue | Purpose |
|-------------|---------|
| [docs/frontend.md § Connected wallet chip — LUNC balance](../docs/frontend.md#connected-wallet-chip-lunc-balance) | Invariants: bank `uluna`, cache key, chip vs menu placement |
| [docs/frontend.md § Connected wallet chip — network & mobile](../docs/frontend.md#connected-wallet-chip-network-mobile) | `shortLabel` on `sm+` trigger, mobile chip vs menu strategy ([#186](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186)) |
| [GitLab #140](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/140) | Balance / copy / explorer / switch wallet (scope **B**: dropdown-only siblings for copy/explorer/switch; **LUNC balance** is in scope) |
| `frontend-dapp/src/hooks/useNativeUlunaBalance.ts` | Shared `['tokenBalance', addr, 'uluna']` query with swap/pool/trade gas gates |
| `frontend-dapp/src/components/wallet/WalletLuncBalance.tsx` | Formatted LUNC line for chip + menu header |

## Rules of thumb

1. **LUNC only in the chip today:** native gas is always **bank `uluna`** (6 decimals, symbol **LUNC**). Do not show CW20 “LUNC-C” here — that is escrow on limit/swap forms.
2. **Reuse `useNativeUlunaBalance`:** do not add a parallel LCD query; invalidate via existing `['tokenBalance']` prefixes after txs.
3. **Visible without opening the menu:** desktop (`sm+`) and mobile chip triggers must include **`WalletLuncBalance`**; the dropdown header repeats balance + full address for copy-friendly QA.
4. **Network on trigger ([#186](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186) — done):** use [`WalletChipNetworkIndicator.tsx`](../frontend-dapp/src/components/wallet/WalletChipNetworkIndicator.tsx) + `getNetworkBadgeCopy()` — logo at all widths, **`shortLabel` text from `sm:`** only. Mobile: balance + truncated address on chip; copy/explorer/switch stay in the menu ([#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185)). `data-testid="wallet-network-short-label"`.
5. **Menu dismiss + Escape ([GitLab #187](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/187) — done):** when the connected dropdown is open, render **`app-menu-dismiss`** (`type="button"`, `aria-label="Close wallet menu"`) — same pattern as [`Layout.tsx`](../frontend-dapp/src/components/common/Layout.tsx) More menu. Register **`Escape`** on `window` only while `showDropdown` is true; do not remove Layout’s More-menu Esc handler.
6. **Siblings (GitLab #140 scope B — filed separately):**
   - [#183](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/183) `CopyButton` primitive — [`AGENTS_FRONTEND_COPY_BUTTON.md`](./AGENTS_FRONTEND_COPY_BUTTON.md) · [docs/frontend.md § CopyButton](../docs/frontend.md#copy-button-primitive)
   - [#184](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184) `getExplorerAddressUrl` — **done**; see [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md)
   - [#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185) dropdown copy / explorer / switch wallet (wire `CopyButton` + `getExplorerAddressUrl` here)
   - [#187](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/187) Esc + semantic menu dismisser — **done** (see rule 5)
   - [#188](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/188) `AddressRow` — **done** for dropdown full address; see [`AGENTS_FRONTEND_ADDRESS_ROW.md`](./AGENTS_FRONTEND_ADDRESS_ROW.md)

## Cross-links

- Address row primitive: [`AGENTS_FRONTEND_ADDRESS_ROW.md`](./AGENTS_FRONTEND_ADDRESS_ROW.md) ([GitLab **#188**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/188))
- Clipboard primitive: [`AGENTS_FRONTEND_COPY_BUTTON.md`](./AGENTS_FRONTEND_COPY_BUTTON.md) ([GitLab **#183**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/183))
- Explorer URL helpers: [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md)
- Connect modal: [`AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md`](./AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md)
- Responsive header shell: [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md)
- Native gas gates (same balance hook): [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)
- Keyboard focus on menu backdrops + shell controls: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md) · [docs/frontend.md § Connected wallet chip](../docs/frontend.md#connected-wallet-chip-lunc-balance) (menu dismiss row)
