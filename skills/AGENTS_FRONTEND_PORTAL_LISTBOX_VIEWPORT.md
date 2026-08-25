# Agent playbook: Portal listbox visual viewport + in-app chrome

Use when changing **`usePortalListbox`**, **`portalListboxPosition.ts`**, **`readPortalListboxViewport`**, **`TokenSearchSelect` / `PairSearchSelect` mobile browse**, `.token-select-dropdown*` CSS, or picker hit targets on phones ([GitLab **#632**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/632)).

Keplr / Station / Cosmostation **in-app browsers** overlay URL chrome on the WebView. Combined with Android IME (`resizes-visual`), `window.innerHeight` stays large while `visualViewport` shrinks — last options sit on the wallet URL bar.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Portal listboxes](../docs/frontend.md#portal-listbox-layout-stability) | **V632-1–V632-8** + #181 / #498 CLS |
| [`portalListboxViewport.ts`](../frontend-dapp/src/lib/portalListboxViewport.ts) | visualViewport + tab + in-app + finger insets |
| [`portalListboxPosition.ts`](../frontend-dapp/src/components/ui/portalListboxPosition.ts) | Flip / clamp; do not force 120px into the reserved band |
| [`PortalListbox.tsx`](../frontend-dapp/src/components/ui/PortalListbox.tsx) | `visualViewport` `resize` / `scroll` + window listeners |
| [`coarseNarrowViewport.ts`](../frontend-dapp/src/lib/coarseNarrowViewport.ts) | Coarse **and** ≤767 → browse-without-IME |
| [`TokenSearchSelect.tsx`](../frontend-dapp/src/components/trade/TokenSearchSelect.tsx) | B2: button trigger; search inside the portal |
| [`PairSearchSelect.tsx`](../frontend-dapp/src/components/trade/PairSearchSelect.tsx) | Same browse path on Trade / Limits |
| [`e2e/swap-token-select-viewport.spec.ts`](../frontend-dapp/e2e/swap-token-select-viewport.spec.ts) | Phone-width clearance + last-option click |

## Invariants (V632-1–V632-8)

1. **V632-1 — Visual viewport, not `innerHeight` alone.** `getPortalListboxViewport()` passes visualViewport width and folds IME occlusion into `bottomInset` / `topInset`. `position: fixed` stays layout-viewport-relative. Do **not** revert `usePortalListbox` to `window.innerHeight` only.
2. **V632-2 — Additive insets.** `bottomInset` = DEX tab bar (`getMobileBottomNavInsetPx`) + visual occluded band + optional in-app chrome (~56px when `detectWalletInAppBrowser().isInAppBrowser` **and** coarse/narrow) + 44px finger gap when any of those apply. Extra inset is safer than overlap. In-app UA is **not** the only fix (Android Chrome + IME must still clear).
3. **V632-3 — First-frame sync + visualViewport listeners.** Keep the render-time `getBoundingClientRect` read. Listen to `visualViewport` `resize` / `scroll` with the same reducer bump as window resize. No second `setState` pass for initial coords (#181 / #498).
4. **V632-4 — Last option cannot sit on chrome.** `computePortalListboxStyle` must not force `PORTAL_LISTBOX_MIN_MENU_HEIGHT` (120) when that would overflow the reserved band. Menu bottom stays above `vh - bottomInset` minus pad.
5. **V632-5 — Browse without IME (B2).** On coarse **and** ≤767px, `TokenSearchSelect` / `PairSearchSelect` open from a **button** `role="combobox"`; search is an explicit field **inside** the portaled menu. Do not auto-focus that field. Desktop (`pointer: fine` or width ≥768) stays today’s input combobox. Do **not** fork a second full-screen sheet unless A+B2 still fail on a 640px WebView (issue preferred B3 only then).
6. **V632-6 — Hit targets.** Coarse pointer: option `min-height` ≥44px. List uses `overscroll-behavior: contain`. Ignore option click if the row moved &gt;12px between pointerdown and click (IME / chrome slide).
7. **V632-7 — Do not hide Keplr chrome / do not “use Chrome only”.** Cannot style in-app URL bars. In-app remains a valid connect path (**WC-M7** / [#554](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/554)). No always-on “tap carefully” essay (#489).
8. **V632-8 — Do not regress picker contracts.** Swap stays a searchable factory-gated combobox (#481). Mint stays button `TokenSelect`. #498 CLS, #350 Enter-first-hit, #244 keys, #562 gem hide, portal `position: fixed` on `document.body` unchanged. Viewport meta stays default `resizes-visual` (no `interactive-widget` until measured). E2E helpers still target `getByRole('combobox', { name: 'Select token you pay|receive' })`; type-to-filter uses the in-menu `searchbox` when the trigger is a button.

## Rules of thumb

1. **Fail closed** on spoofed `visualViewport` (0 / NaN / huge): clamp to a usable menu; never throw; never cover the tab bar.
2. **UA spoof** of `Keplr` only adds inset. It must not unlock gems, skip Legal, or change quotes.
3. **Factory gate / XSS / query cap** stay in the same `tokens` / text-only path. A new search field must keep `TOKEN_SEARCH_MAX_QUERY_LENGTH` (128).
4. After geometry changes, run `make verify-issue-632` and the #498 CLS spec.

## Verification

```bash
make verify-issue-632
# With LocalTerra + deploy env (phone viewport):
cd frontend-dapp && npx playwright test --project=e2e-smoke \
  e2e/swap-token-select-cls.spec.ts \
  e2e/swap-token-select-viewport.spec.ts \
  e2e/trade-pair-select-cls.spec.ts
```

Manual (required — Playwright cannot drive Keplr chrome): Android Keplr in-app, compact + large, 10 fast picks on You Pay and You Receive; last visible token must select, not the URL bar. Then search-from-menu. Android Chrome + IME on the amount field, then open the token menu — list must not sit under Gboard.

## Related

- CLS / fixed portal: [`AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md`](./AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md) ([#181](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/181), [#498](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/498))
- Token search: [`AGENTS_FRONTEND_TOKEN_SEARCH.md`](./AGENTS_FRONTEND_TOKEN_SEARCH.md) ([#481](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/481))
- Keyboard APG: [`AGENTS_FRONTEND_PORTAL_LISTBOX_KEYBOARD.md`](./AGENTS_FRONTEND_PORTAL_LISTBOX_KEYBOARD.md) ([#244](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/244))
- In-app connect path: [`AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md`](./AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md) (**WC-M7**)
