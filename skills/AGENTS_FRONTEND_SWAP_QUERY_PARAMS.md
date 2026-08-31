# Agent playbook: Swap query params on `/` and `/swap` (GitLab #711)

Audience: third-party agents touching Swap routing, share/aggregator deep links, `ust1SecondarySwapPath`, or query-string parsers on the homepage.

**Issue:** [GitLab **#711**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/711)  
**Invariants:** [`docs/frontend.md` § Swap query params](../docs/frontend.md#swap-query-params) (**Q711-1–Q711-8**)  
**Related:** [#182](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/182) shell nav, [#481](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/481) factory gate, [#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562) gems, [#630](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/630) LUNC/USTC, [#678](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/678) quote-only / acquire, [#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508) **U1**, [#542](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/542) **C542-11**, [#596](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/596) hybrid always-on, [#713](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/713) URL rewrite / reverse / Share / Create+Trade prefill.

## Problem class

`/` is Swap. `/swap` was a catch-all `Navigate to="/"` that **dropped** `search` and `hash`. `SwapPage` never read `useSearchParams`, so Uniswap/Pancake `inputCurrency`/`outputCurrency` and Terra DEX `from`/`to` landed on `defaultRetailSwapTokenPair`.

## Invariants (Q711-1–Q711-8)

| ID | Rule |
|----|------|
| **Q711-1** | `/swap` and `/swap/` redirect to `{ pathname: '/', search, hash }` (location object, never concatenated user strings). Swap tab stays `path: '/'` `end: true`. Do **not** mount a second `SwapPage` at `/swap`. |
| **Q711-2** | A resolved id applies only if it is in `getAllTokens(pairs)` (factory + wrap natives). Unlisted CW20s are not injected or quoted. |
| **Q711-3** | `terra1…` must pass `isValidTerraBech32Address`. Hostile / overlong / `javascript:` / `data:` / `http(s):` / `//` / `0x` / `ibc/` / `factory/` / `ETH`/`BNB`/`WETH` → ignore that side. Never echo the raw string into the combobox. |
| **Q711-4** | Production hides gems (`isRetailHiddenTestToken`). Never honor `?showGems=1` (**X8**). LocalTerra / `VITE_SHOW_TEST_TOKENS=true` may apply factory-listed gems. |
| **Q711-5** | Tickers: `LUNC`/`uluna` → `uluna`; `USTC`/`UST`/`uusd` → `uusd`; `CL8Y`/`UST1`/`cLUNC`/`cUSTC`/`vFDUSD`/`USTR` via registry. Display stays LUNC/USTC (**#630**). Spoofed `symbol=UST1` on a gem address does not win (**X1**). |
| **Q711-6** | Optional amount: `exactAmount` / `amount` / `value` / `amountIn` when `isPositiveDecimalAmount` and ≤ 24 chars. `exactField=output` on a **direct pair** is a reverse quote ([#713](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/713)); execute stays offer-in. Never auto-submit, auto-connect, or skip #678 / 5–30–99 / blacklist / pause / freeze. Ignore `slippage`, `expertMode`, `recipient`, `pool_only`, `hybrid_optimize`. |
| **Q711-7** | Apply inbound search once after `allTokens` is ready. Then write canonical `/?from=&to=` on picker/flip/amount (`replace`). Query wins over `defaultRetailSwapTokenPair`. Picker / flip afterward is not re-forced from the **inbound** string. |
| **Q711-8** | Create Pair `/create?a=&b=` prefill and Trade `?from=&to=` resolve are **#713** (**C542-11** / **Q713-6/7**). A Swap deep link to UST1 is AMM, not mint/redeem (**U1**). No lecture banner when a link is ignored (**#489**). |

## Alias families

First key family with a non-empty value wins; last repeated key within a family wins (Charts `?price=`).

| Role | Keys |
|------|------|
| Pay | `inputCurrency`, `from`, `tokenIn`, `token_in`, `sellToken`, `currencyIn`, `inToken`, `pay`, `offer` |
| Receive | `outputCurrency`, `to`, `tokenOut`, `token_out`, `buyToken`, `currencyOut`, `outToken`, `receive`, `ask` |
| Amount | `exactAmount`, `amount`, `value`, `amountIn` |

First-party outbound links use `/?from=&to=` with `uluna` / `uusd` / bech32 (`swapDeepLinkPath` / `ust1SecondarySwapPath`). Uniswap names are inbound-only.

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/swapQueryParams.ts` | Parse, resolve, factory-gate apply, `swapDeepLinkPath` |
| `frontend-dapp/src/components/common/SwapAliasRedirect.tsx` | `/swap` → `/` preserving search + hash |
| `frontend-dapp/src/App.tsx` | Alias routes **before** `*` |
| `frontend-dapp/src/pages/SwapPage.tsx` | Apply once via `useSearchParams` |
| `frontend-dapp/src/utils/ust1SecondaryMarket.ts` | `ust1SecondarySwapPath()` → `/?from=<ust1>&to=<quote>` |
| `frontend-dapp/src/utils/tokenRegistry.ts` | `lookupTokenIdByProductTicker` |

## Do / don’t

- **Do** fail closed to retail defaults per side. Same token both sides: keep pay, default the other.
- **Do** one-sided `?outputCurrency=`: set receive, default pay to the other `defaultRetailSwapTokenPair` id.
- **Do** rewrite the bar to canonical `from`/`to` after apply ([#713](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/713) / [`AGENTS_FRONTEND_SWAP_URL_SYNC.md`](./AGENTS_FRONTEND_SWAP_URL_SYNC.md)).
- **Don’t** map ETH→LUNC. **Don’t** LCD-fetch unlisted CW20s into the picker.
- **Don’t** persist query into `localStorage`.

## Regression

```bash
make verify-issue-711
make verify-issue-678
make verify-issue-542
```

Vitest: `swapQueryParams.test.ts`, `SwapAliasRedirect.test.tsx`, `SwapPage.queryParams.test.tsx`, `ust1SecondaryMarket.test.ts`, `tokenRegistry.test.ts`.  
Playwright (when Vite + factory tokens): `e2e/swap-query-params.spec.ts` plus existing `/swap` viewport/CLS specs.

## Related

- [`AGENTS_FRONTEND_SHELL_NAV.md`](./AGENTS_FRONTEND_SHELL_NAV.md) — Swap tab is `/`; `/swap` preserves search
- [`AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md`](./AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md) — amount prefill is not a submit bypass (**A678**)
- [`AGENTS_UST1_SECONDARY_AMM.md`](./AGENTS_UST1_SECONDARY_AMM.md) — **U1**; `ust1SecondarySwapPath`
- [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) — gems / **X8**
- [`AGENTS_FRONTEND_TOKEN_SEARCH.md`](./AGENTS_FRONTEND_TOKEN_SEARCH.md) — factory-gated picker (#481)
- [`AGENTS_FRONTEND_CREATE_PAIR_PICKER.md`](./AGENTS_FRONTEND_CREATE_PAIR_PICKER.md) — `/create?a=&b=` prefill (#713)
- [`AGENTS_FRONTEND_SWAP_URL_SYNC.md`](./AGENTS_FRONTEND_SWAP_URL_SYNC.md) — rewrite, reverse quotes, Share, Trade resolve
- [`AGENTS_FRONTEND_NATIVE_TICKERS.md`](./AGENTS_FRONTEND_NATIVE_TICKERS.md) — display LUNC/USTC
- [`AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md`](./AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md) — ignore `pool_only` / `hybrid_optimize`
