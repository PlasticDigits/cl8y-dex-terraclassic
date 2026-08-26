# Agent playbook: Swap direction seam plate

Audience: third-party agents editing Swap Pay/Receive chrome, `.swap-direction-btn`, or `.swap-io-stack` paint.

**Issue:** [GitLab **#659**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/659)  
**Invariants:** [`docs/frontend.md` § Swap direction seam](../docs/frontend.md#swap-direction-seam) (**S659-1–S659-8**)  
**Related:** [#488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/488) tokens, [#653](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653) one chrome layer (**C653-5** Swap IO cards stay `card-glass`), [#144](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/144) `:focus-visible`, [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/489) copy density. `/trade` Bid↔Ask / `.limit-side-flip-btn` is **out of scope**.

## Problem class

The switch-directions control sits on the You Pay / You Receive hairline. `--control-surface` is a **glass wash** (~3.5% white in dark). The button is already `z-20`, but the 1px `--chrome-border` **shows through** the fill and across the arrows. Hover `translateY` then re-opens the vacated pixels. This is **paint only** — flip still swaps `fromToken` / `toToken`.

## Do / don’t

- **Do** fill `.swap-direction-btn` with **opaque** `--swap-direction-surface` (seam mid-stop of `--io-stack-bg`) in **both** `theme-dark.css` and `theme-light.css`. Hover uses `--swap-direction-surface-hover`, still opaque.
- **Do** keep a **static** `.swap-direction-seam::before` occluder on the seam (does not move). Do **not** `translate` the plate on hover.
- **Do** keep the Pay/Receive `border-bottom` hairline left and right of the plate. Do **not** delete the seam or split the stack into two floating cards.
- **Do** add `.swap-direction-btn:focus-visible` with `var(--focus-ring)` (same mix as `.limit-side-flip-btn`). `:focus-visible` only.
- **Do** keep wrapper `pointer-events-none` + button `pointer-events-auto`. Wrapper `z-index` stays **20**.
- **Don’t** use `--control-surface` on this control. **Don’t** gold-fill or `*-neo`. **Don’t** wrap the button in `card-glass` / `shell-panel*`.
- **Don’t** change flip JS, quote / hybrid / wrap execute, slippage, or `toRawAmount`. **Don’t** add lecture copy under the flip.
- **Don’t** raise z-index so the plate covers Settings, TermsGate, risk modal, or portaled listboxes ([#632](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/632)).
- **Don’t** drop `.swap-io-stack { overflow: hidden }` to unclip a halo.
- **Don’t** bind token metadata into the button (`dangerouslySetInnerHTML`). Icon stays `aria-hidden`. `aria-label` stays `Swap pay and receive tokens`.

## Invariants

| ID | Meaning |
|----|---------|
| **S659-1** | Dark idle: no `--chrome-border` hairline inside the direction button or across the arrows (375px and 1280px). |
| **S659-2** | Light: same as S659-1. |
| **S659-3** | Hover does not re-open the seam (no plate translate; static occluder stays on the line). |
| **S659-4** | Keyboard Tab shows `--focus-ring`; mouse click does not leave `:focus-visible`. |
| **S659-5** | Click still swaps pay/receive identities; `aria-label` unchanged; wrap/unwrap on `/` uses the same control. |
| **S659-6** | Pay/Receive hairline remains left and right of the button (two halves). |
| **S659-7** | `check_chrome_nesting.py` and `check_design_tokens.py` stay green. No `*-neo`. No extra `card-glass` nest. Plate tokens are opaque `rgb()` in both themes. |
| **S659-8** | Token pickers, Settings, and legal/risk chrome still open above the stack. Flip `z-index` ≤ 20. Hit target stays ~40–44px. |

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/pages/SwapPage.tsx` | `.swap-direction-seam` wrapper + `swap-direction-btn` |
| `frontend-dapp/src/index.css` | `.swap-io-stack*`, opaque plate, static `::before`, `:focus-visible` |
| `frontend-dapp/src/theme-dark.css` / `theme-light.css` | `--swap-direction-surface` / `-hover` |
| `frontend-dapp/src/swapDirectionSeam.test.ts` | Token + CSS + markup guards |
| `frontend-dapp/e2e/swap-direction-seam-659.spec.ts` | Computed-style opaque plate (5 workers, `PLAYWRIGHT_SKIP_CHAIN=1`) |

## Regression

```bash
make verify-issue-659
```

No LocalTerra required. Playwright uses `PLAYWRIGHT_SKIP_CHAIN=1` and a dedicated `PLAYWRIGHT_WEB_PORT`. Do not add `e2e-tx`.

## Related

- [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md) — tokens / no `*-neo` (#488)
- [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) — Swap IO cards stay `card-glass` (**C653-5**)
- [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md) — `:focus-visible` + `--focus-ring` (#144)
- [`AGENTS_FRONTEND_THEME_TOGGLE.md`](./AGENTS_FRONTEND_THEME_TOGGLE.md) — dark/light header toggle
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — no lecture under the flip (#489)
