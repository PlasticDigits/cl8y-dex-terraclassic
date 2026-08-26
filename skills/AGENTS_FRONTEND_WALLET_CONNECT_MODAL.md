# Agent playbook: Connect Wallet modal (extensions + layout + logos + dismiss)

Use when changing wallet list UI, extension detection in the modal, **Install** links, row layout on narrow viewports, **circular wallet logos**, or **Connect / pairing / Expert Mode dismiss overlay** (Close, backdrop, Escape, header toggle).

## Canonical references

| Doc / issue | Purpose |
|-------------|---------|
| [docs/frontend.md § Connect modal: extension install detection](../docs/frontend.md#connect-modal-extension-install) | Invariants: detection signals, **Ready** pill, no **Not installed** pill, truncation, focus/visibility refresh |
| [docs/frontend.md § Connect modal: circular wallet logos](../docs/frontend.md#connect-modal-wallet-logos) | Invariants: local `/wallets/*` assets, a11y, layout vs badges (#490) |
| [docs/frontend.md § Connect modal: dismiss and overlay](../docs/frontend.md#connect-modal-dismiss) | Invariants **D1–D9**: Close, backdrop, Escape, header toggle, cancel-while-connecting, risk gate (#672) |
| [GitLab #139](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/139) | Original install-detection UX |
| [GitLab #160](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/160) | Layout: long names vs badges; removed redundant **Not installed** pill |
| [GitLab #159](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/159) | **Leap** removed from the wallet list (vendor sunset / dead install URL) |
| [GitLab #490](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/490) | Circular brand logos left of each wallet name |
| [GitLab #672](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/672) | Dismiss / overlay: labeled Close, outside click, Escape, header toggle, short-viewport clip |
| `frontend-dapp/src/components/wallet/WalletModal.tsx` | Row markup, **Install** CTA, **Ready** badge, icon slot, `handleClose` vs `isConnecting` |
| `frontend-dapp/src/components/ui/Modal.tsx` | Shared dialog: Close, portal-root dismiss, Escape, `dismissible`, pinned header |
| `frontend-dapp/src/components/wallet/WalletButton.tsx` | Header **Connect Wallet** toggle (`aria-expanded`) |
| `frontend-dapp/src/components/wallet/walletIconSrc.ts` | Local `/wallets/*` path map + helpers |
| `frontend-dapp/src/components/wallet/WalletOptionIcon.tsx` | Circular decorative `<img>` for a row |
| `frontend-dapp/public/wallets/PROVENANCE.md` | Per-file logo source (official kit / product art / original Simulated glyph) |
| `frontend-dapp/src/services/terraclassic/walletExtensionInstall.ts` | `isBrowserWalletExtensionDetected`, `WALLET_EXTENSION_INSTALL_URL` |

## Dismiss invariants (D1–D9, GitLab #672)

1. **D1 — Labeled Close stays on screen.** Dismissible modals show a header button with visible **Close** and a decorative X (`aria-hidden`). Accessible name is `aria-label="Close"` or **Close connect wallet**. Use `.app-modal-close` (`--ink` / `--control-surface` / `--line`), not `btn-muted` alone. Pin `.app-modal-header`; scroll `.app-modal-body`; cap `.app-modal-panel` with `max-height` (mobile: subtract `--app-mobile-nav-stack`). `:focus-visible` ring uses `--focus-ring`. No `*-neo`, no gold fill.
2. **D2 — Dimmed page always dismisses.** Put `onClick` on `.app-modal-portal-root` (and let the backdrop bubble). Do not require the user to hit only the absolutely positioned backdrop sibling. After close, the portal unmounts so Swap / Pay / token pickers receive clicks again.
3. **D3 — Escape + focus trap.** Escape closes when `dismissible`. Trap Tab inside the dialog while it is open (`role="dialog"` / `aria-modal`).
4. **D4 — Panel clicks are not “outside”.** `stopPropagation` on `.app-modal-panel`. Wallet rows, **Install** (`target=_blank`), pairing Open / Copy, and Expert Mode inputs must not dismiss. Allowlist for WC links stays **WC-M5**.
5. **D5 — Header trigger toggles.** Disconnected **Connect Wallet** / **Connect** closes (or cancels) when the dialog is already open; a second click opens again. `aria-haspopup="dialog"` + `aria-expanded`. In-page CTAs (`openWalletModal()`) only open. Do not add a second header close widget. Do **not** raise header `z-index` above the portal ([#181](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/181) / **T527-8**).
6. **D6 — Dismiss while connecting cancels.** Close / backdrop / Escape / pairing **Cancel** / header **Cancel** must call `cancelConnection()` (bump `connectAttemptId`, abort WC, close pairing, clear `isConnecting`). Do not leave the header stuck on **Cancel**. Do not treat cancel as a successful connect (`address` stays null; no `cl8y_wallet_connection`).
7. **D7 — Risk acknowledgement stays blocking.** [`RiskAcknowledgementModal`](../frontend-dapp/src/components/legal/RiskAcknowledgementModal.tsx) is `dismissible={false}` ([#138](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)): no Close, no backdrop, no Escape. Header Connect must not unmount it. See [`AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](./AGENTS_FRONTEND_RISK_DISCLAIMERS.md).
8. **D8 — Pairing stack + clickwrap.** Pairing portal `z-[10001]` above Connect `z-[9999]` (**WC-M8**). Dismiss without connect must not skip TermsGate ([#517](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/517)). Do not lift toasts through the dialog.
9. **D9 — Tests.** Vitest covers Close, backdrop/root, Escape, panel no-op, `dismissible={false}`, connecting cancel, header toggle. `make verify-issue-672`. Optional Playwright: `e2e/navigation.spec.ts` Connect dismiss.

Do **not** bind wallet / pairing `name` into the close control as HTML (`dangerouslySetInnerHTML`).

## Rules of thumb

1. **Missing extension:** communicate with subdued row + **Install** only — do not add a second “not installed” text badge; it competes for horizontal space with **Extension** / **Ready** on mobile.
2. **Long wallet names:** keep **`min-w-0`** on the label flex column, **`truncate`** on the name, and **`title={fullName}`** for hover tooltip.
3. **WalletConnect rows:** never show extension-missing treatment; detection returns “present” for those names by design. Same-device mobile pairing (deep-link + copy, not QR-only) is [#519](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/519) / [#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554) / [#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566) — [`AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](./AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md). On mobile Chrome without the matching extension, Keplr / Station / Cosmostation are WalletConnect rows (not Install-only). **Leap** stays absent ([#159](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/159)).
4. **Logos (#490):** every production row (Station, Keplr, Cosmostation, LuncDash, Galaxy Station) shows a **32px circular** local icon via `WalletOptionIcon`. Prefer **official / product** marks vendored under `public/wallets/`. Do **not** use generic bridge-style placeholder glyphs (globe/star/dollar). If a verified official asset is unavailable, ship a **unique** wallet-specific SVG (not a shared placeholder) and document it in `PROVENANCE.md`. Never hotlink CDNs. Simulated Wallet may use an original non-vendor glyph only.
5. **a11y:** icons are decorative (`alt=""`, `aria-hidden`); keep the button `aria-label` + visible name. Close control: decorative X if the **Close** text is shown; do not rely on a low-contrast glyph alone.
6. **Dismiss only (#672):** do not change option lists, extension detection, Install URLs, logos, hybrid quote, or execute paths when fixing overlay UX.
7. **Before closing #139 / #490 / #672:** run **`npm run build`** and **`npx vitest run`** in `frontend-dapp` — both are required QA gates ([docs/frontend.md § Connect modal](../docs/frontend.md#connect-modal-extension-install), [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md)). For dismiss: `make verify-issue-672`.

## Cross-links

- Dev wallet / bundle safety: [`AGENTS_BUNDLE_DEV_WALLET.md`](./AGENTS_BUNDLE_DEV_WALLET.md)
- Production build / `tsc -b` hygiene: [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md)
- Connected header chip (chain logo, not wallet brand): [`AGENTS_FRONTEND_WALLET_CHIP.md`](./AGENTS_FRONTEND_WALLET_CHIP.md)
- WalletConnect same-device mobile pairing: [`AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](./AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md) ([#519](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/519) / [#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554) / [#566](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/566))
- First-visit risk gate (must stay non-dismissible): [`AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](./AGENTS_FRONTEND_RISK_DISCLAIMERS.md) ([#138](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138))
- Connected Legal clickwrap: [`AGENTS_FRONTEND_CLICKWRAP.md`](./AGENTS_FRONTEND_CLICKWRAP.md) ([#517](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/517))
