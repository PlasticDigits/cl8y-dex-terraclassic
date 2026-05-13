# Agent playbook: Connect Wallet modal (extensions + layout)

Use when changing wallet list UI, extension detection in the modal, **Install** links, or row layout on narrow viewports.

## Canonical references

| Doc / issue | Purpose |
|-------------|---------|
| [docs/frontend.md § Connect modal: extension install detection](../docs/frontend.md#connect-modal-extension-install) | Invariants: detection signals, **Ready** pill, no **Not installed** pill, truncation, focus/visibility refresh |
| [GitLab #139](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/139) | Original install-detection UX |
| [GitLab #160](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/160) | Layout: long names vs badges; removed redundant **Not installed** pill |
| `frontend-dapp/src/components/wallet/WalletModal.tsx` | Row markup, **Install** CTA, **Ready** badge |
| `frontend-dapp/src/services/terraclassic/walletExtensionInstall.ts` | `isBrowserWalletExtensionDetected`, `WALLET_EXTENSION_INSTALL_URL` |

## Rules of thumb

1. **Missing extension:** communicate with subdued row + **Install** only — do not add a second “not installed” text badge; it competes for horizontal space with **Extension** / **Ready** on mobile.
2. **Long wallet names:** keep **`min-w-0`** on the label flex column, **`truncate`** on the name, and **`title={fullName}`** for hover tooltip.
3. **WalletConnect rows:** never show extension-missing treatment; detection returns “present” for those names by design.

## Cross-links

- Dev wallet / bundle safety: [`AGENTS_BUNDLE_DEV_WALLET.md`](./AGENTS_BUNDLE_DEV_WALLET.md)
