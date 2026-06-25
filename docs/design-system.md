# Cyberminimalist Glass System

Authoritative visual spec for the CL8Y DEX frontend. Implementation lives in [`frontend-dapp/src/index.css`](../frontend-dapp/src/index.css) with theme tokens in [`theme-dark.css`](../frontend-dapp/src/theme-dark.css) and [`theme-light.css`](../frontend-dapp/src/theme-light.css). Dual theme is toggled via `data-theme` on `<html>`.

**Agent playbook:** [`skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md`](../skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md) · engineering invariants: [`docs/frontend.md`](./frontend.md) · QA checklist: [`QA_TEMPLATE.md`](../QA_TEMPLATE.md) §10.

## Principles

| Principle | Meaning |
|-----------|---------|
| Glass surfaces | Frosted panels: `backdrop-filter`, warm gradients, soft inset highlights — not flat Tailwind `primary` blue. |
| Token-first | Compose UI from CSS variables (`--ink`, `--line`, `--mint`, …) and `@layer components` primitives below. |
| No parallel systems | Avoid ad-hoc Tailwind color utilities for product chrome; extend tokens or primitives instead. |
| Keyboard focus | Interactive primitives use `:focus-visible` with `var(--focus-ring)` — see [frontend.md § WCAG 2.4.7](./frontend.md#keyboard-focus-visible-wcag-247). |

Legacy **neo-brutalist** class names (`*-neo`) were removed in [GitLab #415](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/415); use the `*-glass` primitives below.

## Color & surface tokens

| Token | Role |
|-------|------|
| `--bg-0`, `--bg-1`, `--bg-2` | Page background stack |
| `--ink`, `--ink-dim`, `--ink-subtle` | Primary, secondary, tertiary text |
| `--line`, `--line-strong` | Borders and dividers |
| `--mint`, `--mint-soft`, `--accent`, `--accent-gradient` | Warm accent (legacy `--mint` name retained) |
| `--panel-bg`, `--panel-bg-strong` | Shell panel gradients |
| `--card-bg` | Inset card surfaces |
| `--control-surface`, `--control-surface-hover` | Inputs, muted buttons, tabs (inactive) |
| `--focus-ring` | Keyboard focus ring color |
| `--shadow-panel`, `--shadow-panel-strong`, `--shadow-card`, `--shadow-button`, `--shadow-button-muted` | Elevation |
| `--color-positive`, `--color-negative`, `--color-warning` | Semantic chart / status (not button fills) |

Theme files override the above per `data-theme='dark'` | `'light'`.

## Typography

| Use | Font | Notes |
|-----|------|-------|
| Headings (`h1`–`h6`) | **Chakra Petch** | Slight letter-spacing |
| Body, controls | **IBM Plex Sans** | Default `body` font |
| Monospace (addresses, order IDs) | Tailwind `font-mono` on fields | Pair with `input-glass` |

Labels use `.label-glass`: uppercase, `text-xs`, semibold, wide tracking.

## Spacing & radii

| Primitive | Padding | Border radius |
|-----------|---------|---------------|
| `shell-panel`, `shell-panel-strong`, `card-glass` | `p-4 sm:p-5` | 24px (`shell-panel`, `card-glass`) / 30px (`shell-panel-strong`) |
| `btn-primary`, `btn-muted` | `px-4 py-2` | 16px |
| `input-glass`, `select-glass`, `token-select-trigger` | `px-4 py-3` | 18px |
| `tab-glass` | `px-4 py-2` | 14px |
| `badge-glass` | `px-2.5 py-1` | pill (`999px`) |

## Component matrix

| Class | When to use | Pair with |
|-------|-------------|-----------|
| `shell-panel` | Page-level sections, swap outer shell | Default route chrome |
| `shell-panel-strong` | Emphasized panels (`Card strong`) | Hero stats, primary workspace |
| `card-glass` | Nested content blocks inside a shell | Order ticket, swap IO rows, tables-in-card |
| `btn-primary` | Primary CTA (submit, confirm) | Optional `btn-cta` for extra elevation |
| `btn-muted` | Secondary / toggle-off / inline actions | Mode toggles, pair-not-found CTA |
| `btn-disabled` | Non-interactive stand-in | Loading or blocked actions |
| `tab-glass` + `tab-glass-active` / `tab-glass-inactive` | Segmented controls (side, slippage, order type) | `role="tablist"` or `radiogroup` as appropriate |
| `input-glass` | Text/number fields | `label-glass` + `htmlFor` |
| `select-glass` | Native `<select>` | Rare; prefer `token-select-*` for assets |
| `label-glass` | Field captions | Always associate with control id |
| `badge-glass` (+ `badge-glass-success` / `-warning` / `-error` / `-accent`) | Status chips | `Badge` component |
| `alert-error`, `alert-success`, `alert-warning`, `alert-info` | Inline notices | Pair with `role="alert"` when assertive |

### React wrappers

| Component | Maps to |
|-----------|---------|
| [`Card`](../frontend-dapp/src/components/ui/Card.tsx) | `card-glass` or `shell-panel-strong` |
| [`Badge`](../frontend-dapp/src/components/ui/Badge.tsx) | `badge-glass*` variants |

### Deprecated

| Class | Status |
|-------|--------|
| `.glass` | **Deprecated** — unused in `src/`. Prefer `shell-panel` (padded, rounded) or `card-glass` for nested blocks. Kept in CSS for backward compatibility only; do not add new usages. |

## Icons & unicode

- Prefer existing SVG assets under `frontend-dapp/public/assets/`.
- Inline unicode (arrows, middots) is acceptable in dense tables when it matches surrounding `text-xs` / `ink-dim` tone.
- Do not mix Material / Heroicons with different stroke weights without aligning to 12–16px and `--ink-dim`.

## Compliance reference pages

| Page | Primitives to verify |
|------|-------------------|
| `/` (Swap) | `shell-panel` wrapper, `card-glass` IO stack, `label-glass`, `tab-glass*` slippage |
| `/limits` | `card-glass` place panel, `btn-primary` / `btn-muted` place-mode toggle |
| `/trade` | `card-glass` chart/order book, `tab-glass*` order tabs |

## Verification

```bash
# No legacy -neo classes
rg '-neo' frontend-dapp/src && exit 1 || echo OK

# Frontend lint + unit tests
make lint-frontend
make test-frontend
```

Manual: `/limits` → Single/Ladder toggle (active = `btn-primary`, inactive = `btn-muted`). `/trade?pair=invalid` → pair-not-found CTA uses `btn-muted`.
