# Agent playbook: form inputs and programmatic labels

Use when adding or editing **text inputs**, **number inputs**, or **settings fields** in `frontend-dapp/src/` so screen readers receive a proper control name.

## Canonical references

| Doc / issue | Purpose |
|-------------|---------|
| [docs/frontend.md § Form inputs — programmatic labels](../docs/frontend.md#form-inputs-programmatic-labels) | Invariants (`htmlFor` + `id`, `useId()`, checkbox/radio wrap pattern) |
| [GitLab #143](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/143) | Source audit sites and verification steps |
| [DEX #133](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/133) | Visual / UX QA umbrella |

## Rules of thumb

1. **Default:** `<label htmlFor={id}>` + `<input id={id}>` with **`useId()`** from React for stable, unique ids per mount (see `LimitOrderEscrowAmountField`, `SwapPage`, `CreatePairPage`).
2. **Shared subcomponents** that already take an `idPrefix` (e.g. `LimitOrderExpiryField`) should keep **deterministic** ids: `` `${idPrefix}-field-name` `` for predictability in tests and docs.
3. **Visually minimal fields** (e.g. search next to a button) may use **`sr-only`** on the label so layout stays unchanged while association remains programmatic.
4. Do **not** rely on `placeholder` alone as the only name; pair it with a label or `aria-label` when fixing a field.

## Related

- Accessibility CI (axe + trade/chart/wallet): [`AGENTS_FRONTEND_A11Y_CI.md`](./AGENTS_FRONTEND_A11Y_CI.md) ([#214](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/214))
- Keyboard focus: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
- Production build / source maps: [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md)
- Responsive header (unrelated a11y class): [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md)
