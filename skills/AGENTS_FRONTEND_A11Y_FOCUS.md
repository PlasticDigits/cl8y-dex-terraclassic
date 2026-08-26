# Agent playbook: keyboard focus visibility (WCAG 2.4.7)

Use when changing **interactive styling** in [`frontend-dapp/src/index.css`](../frontend-dapp/src/index.css), **Swap “You Pay” amount input**, header/nav buttons, primary CTAs, connect-wallet modal rows, or **`:focus` / `:focus-visible`** behavior.

## Canonical references

| Doc / issue | Purpose |
|-------------|---------|
| [docs/frontend.md § Keyboard focus visibility](../docs/frontend.md#keyboard-focus-visible-wcag-247) | Invariants and verification checklist ([GitLab **#144**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/144)) |
| `frontend-dapp/src/index.css` | `--focus-ring`; `.input-glass` / `.btn-*` / shell nav `.app-*` / `.wallet-*` / `.tab-glass*` / `.side-control` / `.wallet-option-card`; `.swap-io-amount-input`; `.swap-direction-btn` ([#659](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/659)) |
| [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md) | Header/nav classes overlap this playbook |

## Invariants

1. **`:focus-visible`, not bare `:focus`** for drawn rings — keyboard users see the indicator; mouse clicks should not leave a persistent ring where the platform supports `:focus-visible` (see `.input-glass`, `.select-glass`, `.token-select-trigger`).
2. **Token select triggers** reserve a transparent `0 0 0 2px` focus ring in the default `box-shadow`; `:focus-visible` only changes ring color so opening the control does not shift layout ([GitLab **#181**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/181), [`AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md`](./AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md)).
2. **Ring token** — visible rings use `color-mix(in srgb, var(--focus-ring) 28%, transparent)` (and matching border tweaks where inputs already had them), aligned with `.input-glass:focus-visible`.
3. **Shell controls** — `.app-nav-link`, `.app-more-trigger`, mobile/footer/menu triggers, `.wallet-trigger` (+ connected variant), `.network-badge`, `.wallet-option-card`, `.tab-glass*`, `.side-control`, and `.btn-primary` / `.btn-muted` / `.btn-cta` composites each have explicit `:focus-visible` rules; **active** nav/badge states stack the **active** `box-shadow` with the same outer ring. Buy/Sell side fills (#563) must **not** drop the ring — see [`AGENTS_FRONTEND_TRADE_TICKET_HEADING.md`](./AGENTS_FRONTEND_TRADE_TICKET_HEADING.md).
4. **Menu backdrops** — **`.app-menu-dismiss`** (shell More menu + connected wallet dropdown; [GitLab **#187**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/187), [`AGENTS_FRONTEND_WALLET_CHIP.md`](./AGENTS_FRONTEND_WALLET_CHIP.md)) uses an **inset** `:focus-visible` ring so keyboard users can reach the dismiss control after opening a menu.
5. **Swap amount field** — the large **You Pay** input uses class **`swap-io-amount-input`** (not `focus:outline-none` alone); styles live next to `.swap-io-stack` in `index.css`.
6. **Swap direction button** — `.swap-direction-btn:focus-visible` uses the same `--focus-ring` mix as `.limit-side-flip-btn`. Do **not** add a bare `:focus` ring. Paint/occlusion of the Pay/Receive seam is [#659](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/659) (`make verify-issue-659`).
7. **Dropdown rows** — `.app-menu-link` and `.wallet-menu-item` use a flat ring (no muted chrome shadow) because parent rules zero their `box-shadow`.

## Related

- Accessibility CI (axe + trade/chart/wallet): [`AGENTS_FRONTEND_A11Y_CI.md`](./AGENTS_FRONTEND_A11Y_CI.md) ([#214](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/214))
- Sister issue (labels / same input): [GitLab **#143**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/143)
- Visual QA umbrella: [GitLab **#133**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/133)
- Slippage preset chips stay a labelled `role="group"`: [`AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md`](./AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md) (`#528`)
- Swap Pay/Receive flip plate: [`AGENTS_FRONTEND_SWAP_DIRECTION_SEAM.md`](./AGENTS_FRONTEND_SWAP_DIRECTION_SEAM.md) ([#659](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/659))
