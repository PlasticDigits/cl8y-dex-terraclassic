# Agent playbook: connected wallet chip (header)

Use when changing **`WalletButton.tsx`**, **`WalletLuncBalance.tsx`**, connected-state dropdown affordances, or header wallet styling in **`index.css`**.

## Canonical references

| Doc / issue | Purpose |
|-------------|---------|
| [docs/frontend.md § Connected wallet chip — LUNC balance](../docs/frontend.md#connected-wallet-chip-lunc-balance) | Invariants: bank `uluna`, cache key, chip vs menu placement |
| [GitLab #140](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/140) | Balance / copy / explorer / switch wallet (scope **B**: dropdown-only siblings for copy/explorer/switch; **LUNC balance** is in scope) |
| `frontend-dapp/src/hooks/useNativeUlunaBalance.ts` | Shared `['tokenBalance', addr, 'uluna']` query with swap/pool/trade gas gates |
| `frontend-dapp/src/components/wallet/WalletLuncBalance.tsx` | Formatted LUNC line for chip + menu header |

## Rules of thumb

1. **LUNC only in the chip today:** native gas is always **bank `uluna`** (6 decimals, symbol **LUNC**). Do not show CW20 “LUNC-C” here — that is escrow on limit/swap forms.
2. **Reuse `useNativeUlunaBalance`:** do not add a parallel LCD query; invalidate via existing `['tokenBalance']` prefixes after txs.
3. **Visible without opening the menu:** desktop (`sm+`) and mobile chip triggers must include **`WalletLuncBalance`**; the dropdown header repeats balance + full address for copy-friendly QA.
4. **Siblings (GitLab #140 scope B — filed separately):**
   - [#183](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/183) `CopyButton` primitive — [`AGENTS_FRONTEND_COPY_BUTTON.md`](./AGENTS_FRONTEND_COPY_BUTTON.md) · [docs/frontend.md § CopyButton](../docs/frontend.md#copy-button-primitive)
   - [#184](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184) `getExplorerAddressUrl` — **done**; see [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md)
   - [#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185) dropdown copy / explorer / switch wallet (wire `CopyButton` + `getExplorerAddressUrl` here)
   - [#186](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186) network shortLabel + mobile chip layout
   - [#187](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/187) Esc + semantic menu dismisser
   - [#188](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/188) `AddressRow` cross-cutting umbrella

## Cross-links

- Clipboard primitive: [`AGENTS_FRONTEND_COPY_BUTTON.md`](./AGENTS_FRONTEND_COPY_BUTTON.md) ([GitLab **#183**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/183))
- Explorer URL helpers: [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md)
- Connect modal: [`AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md`](./AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md)
- Responsive header shell: [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md)
- Native gas gates (same balance hook): [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)
- Keyboard Esc (Layout More menu only today): [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
