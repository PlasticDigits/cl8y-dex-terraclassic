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
4. **Wallet chip** dropdown copy is [#185](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/185); explorer URLs are [#184](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184) — [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md).
5. **Future surfaces:** trader profile, pool/LP addresses, pair chips, `TxResultAlert` tx hash — reuse this primitive.

## Cross-links

- Connected wallet chip: [`AGENTS_FRONTEND_WALLET_CHIP.md`](./AGENTS_FRONTEND_WALLET_CHIP.md) ([GitLab **#140**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/140))
- Terra explorer URLs: [`AGENTS_FRONTEND_TERRA_EXPLORER.md`](./AGENTS_FRONTEND_TERRA_EXPLORER.md) ([GitLab **#184**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/184))
- Retail error funnel (unrelated): [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md)
- Keyboard focus on icon buttons: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
