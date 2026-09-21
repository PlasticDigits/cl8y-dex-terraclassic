# Agent playbook: Swap amount scale for unlisted CW20s (Forgejo #1255)

Audience: third-party agents touching Swap / Trade market **You Pay / You Receive** raw amounts, Max/balance, hybrid book-leg splits, or CW20 `token_info` cache.

**Issue:** [Forgejo **#1255**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1255)  
**Invariants:** [`docs/frontend.md` § Swap amount scaling](../docs/frontend.md#swap-amount-scaling) (**Q1255-1–Q1255-8**)  
**Related:** [#166](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/166) limit LCD fallback · [#564](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/564) Charts never `getDecimals` · **C542-11** tokenlist decimals are not amounts · **QS-3 / QS-4** ([#715](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/715))

## Problem class

Home Swap used `getDecimals` / `swapAmountDecimals` → registry `?? 6`. Factory CW20s missing from `tokenRegistry` (Create Token 6–18, gems when exposed) were quoted and executed as 6-dec. Typed `1` on an 18-dec token simulated `10^6` instead of `10^18` (off by `10^12`). Max/balance inflated `10^18` raw as `1e12`. `fetchCW20TokenInfo` already ran for picker ids but the cache stored only `{symbol,name}`.

This is not #15 (human→raw exists). Do not reopen Charts/tape (#564 / #557).

## Invariants (Q1255)

| ID | Rule |
|----|------|
| **Q1255-1** | Swap `/` and Trade market **execute** amounts use `resolveSwapAssetDecimals` / `useAssetDecimals`. Unknown CW20 is **`null`**, never `?? 6`. |
| **Q1255-2** | Precedence: registry / USTR+USDT pin → valid LCD `token_info.decimals` (wins over a stale indexer 6) → indexer pair-leg / `GET /tokens` while LCD is in flight or transport-failed. Identity is contract/denom, not ticker. |
| **Q1255-3** | Hostile LCD (`255`, `1e9`, `"-1"`, `"18e0"`, non-integer) → unresolved. No indexer fallback. No `10 ** n` explosion. |
| **Q1255-4** | LCD `token_info.decimals` only for picker `getAllTokens` ids (**QS-4**). Do not fetch random pasted `terra1` into the combobox. Do not use `token_info.symbol` as a query key (**X1**). |
| **Q1255-5** | Pending / null: no `toRawAmount` for sim/execute/reverse; CTA **Loading decimals…** / **Token decimals unavailable**. Do not flash a 6-dec quote then snap to 18. |
| **Q1255-6** | Max / balance / insufficient-balance use the same resolved scale. Hide Max until resolved. Raw `10^18` Max is human `1`. |
| **Q1255-7** | Token-info cache key is versioned (`cl8y-dex-token-info-v2`). Old `{symbol,name}` rows are not read as decimals-unknown→6. |
| **Q1255-8** | `getDecimals` / `swapAmountDecimals` still default unknown to **6** for leftover non-execute chrome. Do **not** globally remove that default (#1257 **Q1257-4**). Do not use tokenlist JSON `decimals` as the amount source (**C542-11**). Natives `uluna` / `uusd` stay 6. Unknown bank/IBC stays fail-closed (#630). |

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/swapAssetDecimals.ts` | Parse + precedence (`registry` / `lcd` / `indexer`) |
| `frontend-dapp/src/hooks/useAssetDecimals.ts` | React Query LCD + indexer; `lcdEnabled` = picker id |
| `frontend-dapp/src/utils/tokenDisplay.ts` | `cl8y-dex-token-info-v2` stores `decimals` / `decimalsHostile` |
| `frontend-dapp/src/pages/SwapPage.tsx` | Both legs; reverse ask; hybrid `payDecimals`; Max gated |
| `frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx` | Same resolver for market execute |
| `frontend-dapp/src/utils/swapDisclosure.ts` | `payDecimals === null` → no book split (check **before** `?? getDecimals`) |
| `frontend-dapp/src/utils/limitOrderPriceReference.ts` | Existing LCD path (#166); shares `fetchCW20TokenInfo` |

## Do / don’t

- **Do** fail closed while decimals are in flight (same class as #166 `refResolutionLoading`).
- **Do** keep #678 / 5/30/99 / extra-debit / blacklist / pause / freeze gates.
- **Don’t** LCD-fetch unlisted addresses into the picker. **Don’t** honor `?showGems=1`.
- **Don’t** map ticker spoof (two CW20s named UST1) — identity is contract/denom.
- **Don’t** treat `getDecimals(unknown) === 6` as Swap execute scale. Tests must fail if Swap still does that.
- **Don’t** add every community token to `tokenRegistry` as a substitute for chain decimals.

## Regression

```bash
make verify-issue-1255
```

Vitest: `swapAssetDecimals.test.ts`, `useAssetDecimals.test.tsx`, `SwapPage.assetDecimals.test.tsx`, `tokenDisplay.test.ts` (v2 cache), `swapQuoteAmountScale.test.ts` (chrome still 6).

## Related

- [`AGENTS_FRONTEND_SWAP_TOKENLIST_SYMBOLS.md`](./AGENTS_FRONTEND_SWAP_TOKENLIST_SYMBOLS.md) — **QS-3** unlisted factory CW20 is still a picker id; **QS-4** LCD decimals vs LCD symbol
- [`AGENTS_FRONTEND_SWAP_USTR_USDT_SCALE.md`](./AGENTS_FRONTEND_SWAP_USTR_USDT_SCALE.md) — listed 18-dec pin; unknown chrome stays 6
- [`AGENTS_LIMIT_PRICE_DECIMALS.md`](./AGENTS_LIMIT_PRICE_DECIMALS.md) — #166 fail-closed LCD for limits
- [`AGENTS_FRONTEND_CREATE_PAIR_PICKER.md`](./AGENTS_FRONTEND_CREATE_PAIR_PICKER.md) — **C542-11** tokenlist decimals ≠ amounts
