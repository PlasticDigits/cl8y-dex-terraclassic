# Agent playbook: Portal listbox keyboard (WAI-ARIA APG)

Use when changing **`MenuSelect`**, **`TokenSelect`**, **`usePortalListboxKeyboard`**, **`portalListboxKeyboard.ts`**, or listbox a11y regressions ([GitLab **#244**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/244), gap **M5**; broader audit **#214**).

For **Swap** searchable token combobox (`TokenSearchSelect`) and **Trade/Limits** pair combobox (`PairSearchSelect`), see [`AGENTS_FRONTEND_TOKEN_SEARCH.md`](./AGENTS_FRONTEND_TOKEN_SEARCH.md) and [docs/frontend.md § Token search](../docs/frontend.md#token-search-combobox) / [§ Pair search](../docs/frontend.md#pair-search-combobox) — those use an **input** combobox + inline key handling, **not** `usePortalListboxKeyboard`.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Portal listboxes — layout stability](../docs/frontend.md#portal-listbox-layout-stability) | Keyboard invariants alongside CLS/portal rules |
| [`usePortalListboxKeyboard.ts`](../frontend-dapp/src/components/ui/usePortalListboxKeyboard.ts) | Shared Arrow/Home/End/typeahead/Enter/Escape handling |
| [`portalListboxKeyboard.ts`](../frontend-dapp/src/components/ui/portalListboxKeyboard.ts) | Pure typeahead + index helpers (unit-tested) |
| [`PortalListbox.tsx`](../frontend-dapp/src/components/ui/PortalListbox.tsx) | Positioning + outside click only (no keyboard) |
| [`MenuSelect.keyboard.test.tsx`](../frontend-dapp/src/components/ui/__tests__/MenuSelect.keyboard.test.tsx) | Vitest keyboard interaction regression |
| [`TokenSelect.keyboard.test.tsx`](../frontend-dapp/src/components/ui/__tests__/TokenSelect.keyboard.test.tsx) | Token symbol typeahead regression (Mint) |

## Rules of thumb

1. **Extend the shared hook** — do not duplicate keyboard logic in `TokenSelect` vs `MenuSelect`; both consume `usePortalListboxKeyboard`.
2. **APG listbox pattern** — while open, focus the portaled `<ul role="listbox">` (`tabIndex={-1}`) and drive highlight via **`aria-activedescendant`** on option ids (`{listId}-option-{index}`).
3. **Typeahead** — case-insensitive **prefix** on `getTypeaheadLabel` (MenuSelect: `option.label`; TokenSelect: cached symbol via `getCachedTokenSymbol`). Buffer clears after **500ms** (`PORTAL_LISTBOX_TYPEAHEAD_RESET_MS`). **`openRef`** keeps multi-char typeahead intact between the first printable key and React `open` commit (GitLab **#244** regression).
4. **Preserve** click selection, Escape dismiss, portal positioning (`usePortalListbox`), and CLS invariants from [`AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md`](./AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md).
5. **Visual active row** — `.token-select-option-keyboard-active` (distinct from `.token-select-option-active` selected state).
6. **Guard duplicate select** — `selectingRef` prevents Enter key-repeat from spamming `onChange`.
7. **Pair / token search combobox ([#350](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/350), [#481](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/481))** — `PairSearchSelect` / `TokenSearchSelect`: empty query may prepend the current value so Enter re-selects it; **typed query** must highlight index **0** (first hit) and **not** prepend the current value. Regressions: `PairSearchSelect.issue350.test.tsx`, `TokenSearchSelect.test.tsx`.

## Verification

```bash
make test-frontend
# or targeted:
cd frontend-dapp && npm test -- src/components/ui/__tests__/MenuSelect.keyboard.test.tsx src/components/ui/__tests__/TokenSelect.keyboard.test.tsx src/components/ui/__tests__/portalListboxKeyboard.test.ts
cd frontend-dapp && npm test -- src/components/trade/__tests__/TokenSearchSelect.test.tsx
```

Manual: Tab to Mint token select → open with ArrowDown → navigate with arrows → type symbol prefix → Enter; Escape returns focus to trigger. Swap: focus pay/receive **combobox** → type to filter → Enter (see [`AGENTS_FRONTEND_TOKEN_SEARCH.md`](./AGENTS_FRONTEND_TOKEN_SEARCH.md)).

## Related

- Swap token search: [`AGENTS_FRONTEND_TOKEN_SEARCH.md`](./AGENTS_FRONTEND_TOKEN_SEARCH.md)
- Layout / CLS: [`AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md`](./AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md)
- Focus rings: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
- axe E2E gate: [`AGENTS_FRONTEND_A11Y_CI.md`](./AGENTS_FRONTEND_A11Y_CI.md)
