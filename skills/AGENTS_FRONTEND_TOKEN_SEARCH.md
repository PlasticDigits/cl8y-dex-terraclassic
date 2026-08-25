# Agent playbook: Swap token search combobox

Use when changing **Swap** pay/receive token pickers, [`TokenSearchSelect`](../frontend-dapp/src/components/trade/TokenSearchSelect.tsx), [`tokenSearchQuery.ts`](../frontend-dapp/src/utils/tokenSearchQuery.ts), or E2E helpers that open those controls ([GitLab **#481**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/481), mobile CLS [**#498**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/498)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Token search combobox](../docs/frontend.md#token-search-combobox) | Invariants (factory gate, client filter, a11y, XSS/DoS, #498 layout) |
| [docs/frontend.md § Portal listboxes — layout stability](../docs/frontend.md#portal-listbox-layout-stability) | Leading logo / padding / `queryDraft` CLS rules (#498) |
| [docs/frontend.md § Pair search combobox](../docs/frontend.md#pair-search-combobox) | UX twin on Trade/Limits (`PairSearchSelect`) |
| [`TokenSearchSelect.tsx`](../frontend-dapp/src/components/trade/TokenSearchSelect.tsx) | Swap combobox UI |
| [`tokenSearchQuery.ts`](../frontend-dapp/src/utils/tokenSearchQuery.ts) | Debounce/min-chars/cap + local haystack filter |
| [`TokenSelect.tsx`](../frontend-dapp/src/components/ui/TokenSelect.tsx) | **Mint only** — button + typeahead, no visible search |
| [`e2e/helpers/token-select.ts`](../frontend-dapp/e2e/helpers/token-select.ts) | Playwright: Swap uses `role=combobox` |
| [`e2e/swap-token-select-cls.spec.ts`](../frontend-dapp/e2e/swap-token-select-cls.spec.ts) | Phone-width open/close CLS (#498) |

## Rules of thumb

1. **Swap = `TokenSearchSelect`; Mint = `TokenSelect`** — do not force Mint into a combobox; do not regress Swap back to scroll-only `TokenSelect`.
2. **Factory gate only** — options must stay within `getAllTokens(pairs)` (plus existing native-wrap enrichment). Never feed arbitrary indexer/external token lists into Swap selection.
3. **Client-side filter first** — do not block Swap on indexer availability. There is **no** `GET /api/v1/tokens?q=` today; if added later, escape `ILIKE` wildcards per #459 and still intersect with the factory set.
4. **Do not use `getPairs(q)` for Swap tokens** — pair-shaped results duplicate and are the wrong shape for a token picker.
5. **Keyboard (#350 parity)** — empty query may keep current token at index 0; **typed** ready query highlights index 0 (first hit) and must **not** prepend the current token over the first hit.
6. **Security** — render symbol/name as text only; logos via `resolveTrustedTokenLogoUrl`; truncate long paste (`TOKEN_SEARCH_MAX_QUERY_LENGTH`); `onChange` only for ids in the gated options list; honor `excludeToken`.
7. **Mobile CLS (#498)** — keep leading logo + `.token-select-trigger--with-leading-logo` while open; do not clear the input to empty on focus (`queryDraft` stays `null` until edit); select-all on focus so typing replaces the label.
8. **E2E** — target `getByRole('combobox', { name: 'Select token you pay|receive' })`. Mint still uses `button` / `TokenSelect`.

## Verification

```bash
cd frontend-dapp && npm test -- src/utils/__tests__/tokenSearchQuery.test.ts src/components/trade/__tests__/TokenSearchSelect.test.tsx
# Mint regression (button listbox unchanged):
cd frontend-dapp && npm test -- src/components/ui/__tests__/TokenSelect.keyboard.test.tsx
# With LocalTerra + deploy env (phone viewport CLS):
cd frontend-dapp && npx playwright test e2e/swap-token-select-cls.spec.ts --project=e2e-smoke
```

Manual: open Swap at ~390px width → tap pay combobox → confirm logo/padding stay put and label does not blank until typing; pick another token → trigger width/layout stable; Escape close → no scroll jump. Trade market ticket does not use this control (pair-bound tokens).

## Related

- Portal CLS / CSS: [`AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md`](./AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md) ([GitLab **#181**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/181), **#498**)
- Button listbox keyboard (Mint / MenuSelect): [`AGENTS_FRONTEND_PORTAL_LISTBOX_KEYBOARD.md`](./AGENTS_FRONTEND_PORTAL_LISTBOX_KEYBOARD.md)
- Swap route display (unrelated to picker): [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md)
- Pair/token empty-browse catalog (economic first, gems last): [`AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](./AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) ([GitLab **#534**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534))
- Production hide of gems from Swap browse: [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) ([GitLab **#562**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562))
- Create Pair listed-CW20 catalog (do **not** reuse Swap’s factory universe): [`AGENTS_FRONTEND_CREATE_PAIR_PICKER.md`](./AGENTS_FRONTEND_CREATE_PAIR_PICKER.md) ([GitLab **#542**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/542))
- Native LUNC / USTC visible tickers (never `uluna` / `uusd`): [`AGENTS_FRONTEND_NATIVE_TICKERS.md`](./AGENTS_FRONTEND_NATIVE_TICKERS.md) ([GitLab **#630**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/630)); `make verify-issue-630`
