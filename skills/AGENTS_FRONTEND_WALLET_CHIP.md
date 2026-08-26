# Agent playbook: connected wallet chip (header)

Use when changing **`WalletButton.tsx`**, **`WalletDropdownMenuItems.tsx`**, **`WalletChipNetworkIndicator.tsx`**, **`WalletLuncBalance.tsx`**, connected-state dropdown affordances, or header wallet styling in **`index.css`**.

## Canonical references

| Doc / issue | Purpose |
|-------------|---------|
| [docs/frontend.md § Connected wallet chip — LUNC balance](../docs/frontend.md#connected-wallet-chip-lunc-balance) | Invariants: bank `uluna`, cache key, chip vs menu placement |
| [docs/frontend.md § Connected wallet chip — network & mobile](../docs/frontend.md#connected-wallet-chip-network-mobile) | `shortLabel` on `sm+` trigger, mobile chip vs menu strategy ([#186](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186)) |
| [docs/frontend.md § Connected wallet dropdown](../docs/frontend.md#connected-wallet-dropdown) | Copy / explorer / switch wallet menu rows ([#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185)) + row layout (**W671-1–W671-8**, [#671](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/671)) |
| [GitLab #140](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/140) | Balance / copy / explorer / switch wallet (scope **B**) |
| [GitLab #671](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/671) | Connected dropdown icon+label alignment + header address wrap |
| `frontend-dapp/src/hooks/useNativeUlunaBalance.ts` | Shared `['tokenBalance', addr, 'uluna']` query with swap/pool/trade gas gates |
| `frontend-dapp/src/components/wallet/WalletLuncBalance.tsx` | Formatted LUNC line for chip + menu header |
| `frontend-dapp/src/components/wallet/WalletDropdownMenuItems.tsx` | Labeled menu rows: Copy address, View on explorer, Switch wallet |

## Rules of thumb

1. **LUNC only in the chip today:** native gas is always **bank `uluna`** (6 decimals, symbol **LUNC**). Do not show CW20 **cLUNC** here — that is escrow on limit/swap forms (GitLab #507).
2. **Reuse `useNativeUlunaBalance`:** do not add a parallel LCD query; invalidate via existing `['tokenBalance']` prefixes after txs.
3. **Visible without opening the menu:** desktop (`sm+`) and mobile chip triggers must include **`WalletLuncBalance`**; the dropdown header repeats balance + **`AddressRow`** (`nowrap` truncated label; full bech32 in `title` / copy / explorer — **W671-2**).
4. **Network on trigger ([#186](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/186) — done):** use [`WalletChipNetworkIndicator.tsx`](../frontend-dapp/src/components/wallet/WalletChipNetworkIndicator.tsx) + `getNetworkBadgeCopy()` — logo at all widths, **`shortLabel` text from `sm:`** only. Mobile: balance + truncated address on chip; labeled copy/explorer/switch rows stay in the menu ([#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185)). `data-testid="wallet-network-short-label"`.
5. **Menu dismiss + Escape ([#187](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/187) — done):** when open, render **`app-menu-dismiss`** and register **`Escape`** on `window` only while `showDropdown` is true.
6. **Dropdown stacking:** `.wallet-menu` lives under the sticky header; the environment ribbon is in the **footer**, so menus no longer compete with an under-header ribbon band (historical [#486](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/486)). See [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md). Do **not** raise `.wallet-menu` `z-index` above modal portals (**W671-7**).
7. **Dropdown menu rows ([#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185) — done):**
   - **Copy address** — `CopyButton` with `menuLabel="Copy address"`.
   - **View on explorer** — `getExplorerAddressUrl` then `isSafeExplorerHref`; hide when `null`.
   - **Switch wallet** — `disconnect()` then `setWalletModalOpen(true)`; portal `WalletModal` while connected.
   - Keep **My Portfolio**, **Trader profile**, and **Disconnect** in `WalletButton` (`traderProfilePath` omits invalid bech32).
8. **Do not restyle `WalletModal` connect cards** for this menu. The broken surface is `.wallet-menu` after the connected chip is opened — not `wallet-option-card`.

## Layout invariants (GitLab #671)

| ID | Rule |
|----|------|
| **W671-1** | Every connected-menu action is a **single horizontal row**: icon left, label vertically centered, 8px gap. Own this on **`.wallet-menu-item`** (`inline-flex`, `align-items: center`, `flex-wrap: nowrap`). **Forbid** stacking icons above labels. Do not copy Tailwind `inline-flex items-center gap-2` onto each row. |
| **W671-2** | Header address is truncated + `nowrap` icon cluster. Do **not** use `showFull` + `break-all` in this 210px panel. Full bech32 remains copyable. No lecture copy (#489). |
| **W671-3** | Menu order and `data-testid`s stay: Copy → Explorer → Switch → Portfolio → Trader → Disconnect. Duplicate header copy/explorer icons are OK. |
| **W671-4** | `CopyButton` `menuLabel` stays `role="menuitem"` + `.wallet-menu-item`. Icon-only and `buttonLabel` pairing copy are **not** menuitems. |
| **W671-5** | Explorer `href` is http(s) only (`isSafeExplorerHref`). Trader path is `/trader/{bech32}` or omitted. Portfolio is `/portfolio`. |
| **W671-6** | Keep `#187` / `#214` / `#144`: dismiss, Escape, first-item focus, `:focus-visible`. Icons stay `w-4 h-4`. |
| **W671-7** | Do not raise menu `z-index` above connect/terms portals. Menu `max-height` + overflow stays. Mobile `z-index: 55` is already the tab-bar exception — do not bump it. |
| **W671-8** | Tokens `--ink*` / `--menu-bg` / `--line`. No `*-neo`. No nested `card-glass` / `shell-panel`. Light + dark. |

Regression: `make verify-issue-671` (Vitest + CSS/docs grep; Playwright `navigation.spec.ts` wallet geometry when LocalTerra is up, 5 workers, no `e2e-tx`).

## Cross-links

- Address row (header): [`AGENTS_FRONTEND_ADDRESS_ROW.md`](./AGENTS_FRONTEND_ADDRESS_ROW.md) ([GitLab **#188**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/188) / [#671](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/671))
- Clipboard primitive: [`AGENTS_FRONTEND_COPY_BUTTON.md`](./AGENTS_FRONTEND_COPY_BUTTON.md) ([GitLab **#183**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/183))
- Explorer URL helpers: [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md) ([GitLab **#184**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184))
- Connect modal: [`AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md`](./AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md)
- Responsive header shell (menu stacking **#486**): [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md)
- Native gas gates (same balance hook): [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)
- Keyboard focus: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
- Connected dropdown layout ([#671](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/671)): this playbook **W671-1–W671-8**; `make verify-issue-671`
- Retail copy (no lectures): [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md)
