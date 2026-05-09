# Agent playbook: keyboard focus visibility (WCAG 2.4.7)

Use when changing **interactive styling** in [`frontend-dapp/src/index.css`](../frontend-dapp/src/index.css), **Swap “You Pay” amount input**, header/nav buttons, primary CTAs, connect-wallet modal rows, or **`:focus` / `:focus-visible`** behavior.

## Canonical references

| Doc / issue | Purpose |
|-------------|---------|
| [docs/frontend.md § Keyboard focus visibility](../docs/frontend.md#keyboard-focus-visible-wcag-247) | Invariants and verification checklist ([GitLab **#144**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/144)) |
| `frontend-dapp/src/index.css` | `--focus-ring`; `.input-neo` / `.btn-*` / shell nav `.app-*` / `.wallet-*` / `.tab-neo*` / `.wallet-option-card`; `.swap-io-amount-input` |
| [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md) | Header/nav classes overlap this playbook |

## Invariants

1. **`:focus-visible`, not bare `:focus`** for drawn rings — keyboard users see the indicator; mouse clicks should not leave a persistent ring where the platform supports `:focus-visible` (see `.input-neo`, `.select-neo`, `.token-select-trigger`).
2. **Ring token** — visible rings use `color-mix(in srgb, var(--focus-ring) 28%, transparent)` (and matching border tweaks where inputs already had them), aligned with `.input-neo:focus-visible`.
3. **Shell controls** — `.app-nav-link`, `.app-more-trigger`, mobile/footer/menu triggers, `.wallet-trigger` (+ connected variant), `.network-badge`, `.wallet-option-card`, `.tab-neo*`, and `.btn-primary` / `.btn-muted` / `.btn-cta` composites each have explicit `:focus-visible` rules; **active** nav/badge states stack the **active** `box-shadow` with the same outer ring.
4. **Swap amount field** — the large **You Pay** input uses class **`swap-io-amount-input`** (not `focus:outline-none` alone); styles live next to `.swap-io-stack` in `index.css`.
5. **Dropdown rows** — `.app-menu-link` and `.wallet-menu-item` use a flat ring (no muted chrome shadow) because parent rules zero their `box-shadow`.

## Related

- Sister issue (labels / same input): [GitLab **#143**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/143)
- Visual QA umbrella: [GitLab **#133**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/133)
