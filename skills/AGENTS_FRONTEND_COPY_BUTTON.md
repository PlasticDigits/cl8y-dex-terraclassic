# Agent playbook: `CopyButton` clipboard primitive

Use when adding **copy address / contract / tx hash** affordances anywhere in the dApp ([GitLab **#183**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/183)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Copy to clipboard — CopyButton](../docs/frontend.md#copy-button-primitive) | Invariants: single API path, aria-live, retail strings |
| [`CopyButton.tsx`](../frontend-dapp/src/components/ui/CopyButton.tsx) | Accessible icon button + live region |
| [`copyToClipboard.ts`](../frontend-dapp/src/utils/copyToClipboard.ts) | Injectable `writeText` for Vitest |
| [`copyButtonCopy.ts`](../frontend-dapp/src/utils/copyButtonCopy.ts) | Success/failure retail strings |

## Rules of thumb

1. **Do not** call `navigator.clipboard` directly in pages or wallet components — import **`CopyButton`** or **`copyToClipboard`**.
2. **Always** pass a specific **`ariaLabel`** (e.g. `"Copy wallet address"`, not `"Copy"`).
3. **Empty/whitespace** `text` must fail gracefully (handler returns failure message; no throw).
4. **Wallet chip** **Copy address** row uses **`menuLabel`** ([#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185) — [docs § Connected wallet dropdown](../docs/frontend.md#connected-wallet-dropdown)); layout is owned by **`.wallet-menu-item`** CSS ([#671](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/671) **W671-1 / W671-4**) — do not re-add Tailwind `inline-flex items-center gap-2` on that branch. Explorer menu row uses [#184](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184) — [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md). WalletConnect **Copy pairing link** uses **`buttonLabel`** (not `menuLabel` / `role="menuitem"`) ([#519](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/519) — [`AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](./AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md)).
5. **Address surfaces:** prefer [`AddressRow`](./AGENTS_FRONTEND_ADDRESS_ROW.md) (copy + shorten + explorer) over bare `CopyButton` in pages ([#188](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/188)). Token legs on Pool / Trade / Charts use [`TokenIdentity`](./AGENTS_FRONTEND_TOKEN_IDENTITY.md) ([#541](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/541)). Trader-as-person copy still goes through `AddressRow` (full bech32) — never the 4/6 chip ([#656](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/656), [`AGENTS_FRONTEND_TRADER_IDENTITY.md`](./AGENTS_FRONTEND_TRADER_IDENTITY.md)).
6. **Share URL fallback:** [`ShareLinkButton`](./AGENTS_FRONTEND_SHARE_LINK.md) clipboard fallback must call **`copyToClipboard`** — do not add a second `navigator.clipboard.writeText` path ([#665](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/665)).
7. **Future surfaces:** `TxResultAlert` tx hash copy — extend `AddressRow` or add a sibling row primitive.

## Cross-links

- Address row (copy + explorer): [`AGENTS_FRONTEND_ADDRESS_ROW.md`](./AGENTS_FRONTEND_ADDRESS_ROW.md) ([GitLab **#188**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/188))
- Token identity (Pool / Trade / Charts): [`AGENTS_FRONTEND_TOKEN_IDENTITY.md`](./AGENTS_FRONTEND_TOKEN_IDENTITY.md) ([#541](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/541))
- Trader identity (4/6 + blockie): [`AGENTS_FRONTEND_TRADER_IDENTITY.md`](./AGENTS_FRONTEND_TRADER_IDENTITY.md) ([#656](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/656))
- Connected wallet chip: [`AGENTS_FRONTEND_WALLET_CHIP.md`](./AGENTS_FRONTEND_WALLET_CHIP.md) ([GitLab **#140**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/140) / [**#671**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/671))
- Terra explorer URLs: [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md) ([GitLab **#184**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184))
- Retail error funnel (unrelated): [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md)
- Keyboard focus on icon buttons: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
- WalletConnect pairing copy: [`AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](./AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md) ([#519](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/519))
- Profile Share (URL, not address): [`AGENTS_FRONTEND_SHARE_LINK.md`](./AGENTS_FRONTEND_SHARE_LINK.md) ([#665](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/665))
