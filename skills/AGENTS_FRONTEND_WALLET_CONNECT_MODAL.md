# Agent playbook: Connect Wallet modal (extensions + layout + logos)

Use when changing wallet list UI, extension detection in the modal, **Install** links, row layout on narrow viewports, or **circular wallet logos**.

## Canonical references

| Doc / issue | Purpose |
|-------------|---------|
| [docs/frontend.md § Connect modal: extension install detection](../docs/frontend.md#connect-modal-extension-install) | Invariants: detection signals, **Ready** pill, no **Not installed** pill, truncation, focus/visibility refresh |
| [docs/frontend.md § Connect modal: circular wallet logos](../docs/frontend.md#connect-modal-wallet-logos) | Invariants: local `/wallets/*` assets, a11y, layout vs badges (#490) |
| [GitLab #139](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/139) | Original install-detection UX |
| [GitLab #160](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/160) | Layout: long names vs badges; removed redundant **Not installed** pill |
| [GitLab #159](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/159) | **Leap** removed from the wallet list (vendor sunset / dead install URL) |
| [GitLab #490](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/490) | Circular brand logos left of each wallet name |
| `frontend-dapp/src/components/wallet/WalletModal.tsx` | Row markup, **Install** CTA, **Ready** badge, icon slot |
| `frontend-dapp/src/components/wallet/walletIconSrc.ts` | Local `/wallets/*` path map + helpers |
| `frontend-dapp/src/components/wallet/WalletOptionIcon.tsx` | Circular decorative `<img>` for a row |
| `frontend-dapp/public/wallets/PROVENANCE.md` | Per-file logo source (official kit / product art / original Simulated glyph) |
| `frontend-dapp/src/services/terraclassic/walletExtensionInstall.ts` | `isBrowserWalletExtensionDetected`, `WALLET_EXTENSION_INSTALL_URL` |

## Rules of thumb

1. **Missing extension:** communicate with subdued row + **Install** only — do not add a second “not installed” text badge; it competes for horizontal space with **Extension** / **Ready** on mobile.
2. **Long wallet names:** keep **`min-w-0`** on the label flex column, **`truncate`** on the name, and **`title={fullName}`** for hover tooltip.
3. **WalletConnect rows:** never show extension-missing treatment; detection returns “present” for those names by design.
4. **Logos (#490):** every production row (Station, Keplr, Cosmostation, LuncDash, Galaxy Station) shows a **32px circular** local icon via `WalletOptionIcon`. Prefer **official / product** marks vendored under `public/wallets/`. Do **not** use generic bridge-style placeholder glyphs (globe/star/dollar). If a verified official asset is unavailable, ship a **unique** wallet-specific SVG (not a shared placeholder) and document it in `PROVENANCE.md`. Never hotlink CDNs. Simulated Wallet may use an original non-vendor glyph only.
5. **a11y:** icons are decorative (`alt=""`, `aria-hidden`); keep the button `aria-label` + visible name.
6. **Before closing #139 / #490:** run **`npm run build`** and **`npx vitest run`** in `frontend-dapp` — both are required QA gates ([docs/frontend.md § Connect modal](../docs/frontend.md#connect-modal-extension-install), [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md)).

## Cross-links

- Dev wallet / bundle safety: [`AGENTS_BUNDLE_DEV_WALLET.md`](./AGENTS_BUNDLE_DEV_WALLET.md)
- Production build / `tsc -b` hygiene: [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md)
- Connected header chip (chain logo, not wallet brand): [`AGENTS_FRONTEND_WALLET_CHIP.md`](./AGENTS_FRONTEND_WALLET_CHIP.md)
