# Agent playbook: Swap tokenlist symbols, unique-ticker CI, Share logos (GitLab #715)

Audience: third-party agents touching Swap `from=`/`to=`, Share URLs, `tokenlist.json`, or Create Pair catalog overlays.

**Issue:** [GitLab **#715**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/715)  
**Parent inbound parse:** [#711](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/711) / [`AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md`](./AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md) (**Q711-1–Q711-8**)  
**Parent rewrite / Share:** [#713](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/713) / [`AGENTS_FRONTEND_SWAP_URL_SYNC.md`](./AGENTS_FRONTEND_SWAP_URL_SYNC.md) (**Q713-1–Q713-10**)  
**Invariants:** [`docs/frontend.md` § Swap tokenlist symbols](../docs/frontend.md#swap-tokenlist-symbols) (**TL-1–TL-2**, **QS-1–QS-6**, **SH-1–SH-3**)

Land as a **new MR on top of #713**. Do not reopen #711/#713.

## Problem class

#711/#713 share links wrote execute ids (`uluna`, 64-char `terra1…`). Tickers were a hand-maintained `CW20_MAP`, not [`tokenlist/tokenlist.json`](../tokenlist/tokenlist.json). Duplicate tickers would make `from=UST1` attacker-chosen. Swap Share said **Share** with no pair logos.

## Invariants (TL / QS / SH)

| ID | Rule |
|----|------|
| **TL-1** | `tokenlist.json` symbols are unique case-insensitively (ASCII fold: `UST1` ≡ `ust1`). Execute ids unique (native denom lowercase; CW20 address lowercase). Empty / whitespace / non-ASCII tickers fail CI. Do **not** skip duplicates at runtime. |
| **TL-2** | README states uniqueness and that Swap `from`/`to` / Share use those symbols. Gems stay out of the JSON (**#562** / **U6**). |
| **QS-1** | Inbound `from=UST1` / `from=ust1` / published mixed case selects the overlay-or-published execute id after factory gate. Same for every current tokenlist row in `getAllTokens`. |
| **QS-2** | Checksummed `terra1` and `uluna`/`uusd` still apply. After apply, rewrite to the published symbol when unique (`from=uluna` → `from=LUNC`). |
| **QS-3** | Factory-listed CW20 **not** in the tokenlist stays bech32 outbound (no invented ticker). |
| **QS-4** | Unlisted / gem / hostile / spoofed ticker ignored per side; raw string never echoed. `?showGems=1` inert. LCD `token_info.symbol` is **not** the query key (**X1**). |
| **QS-5** | `/swap?from=UST1&to=cUSTC` preserves search on redirect to `/`. `/trade?from=UST1&to=cUSTC` still `replace`s to `/trade/{uniquePair}`. |
| **QS-6** | `swapDeepLinkPath` / `ust1SecondarySwapPath` / Share URL use symbols when unique. Amount / `exactField=output` unchanged. Execute stays offer-in. Overlay address still shares as `UST1`, not LocalTerra bech32. |
| **SH-1** | Swap header Share visible label is **Share {pay TokenLogo} → {receive TokenLogo}** (`data-testid="swap-share-link"`). Logos update on picker/flip. `aria-label` is text with both display symbols. Logos `alt=""`. |
| **SH-2** | Trader / Portfolio Share stay the word **Share** with no pair logos. No extra `shell-panel*` / `card-glass` (**C653**). |
| **SH-3** | Copied / shared URL is `{origin}/?from=<symbol-or-id>&to=<symbol-or-id>` (+ amount/exactField), never `location.href`, never a gem. |

## Resolve / encode

1. Hostile / overlong / `javascript:` / `data:` / `http(s):` / `//` / `0x` / `ibc/` / `factory/` / `ETH`/`BNB`/`WETH` → ignore that side.
2. Native denom `uluna` / `uusd`.
3. Inbound-only alias `UST` → `uusd` (not a tokenlist row).
4. Unique tokenlist symbol (ASCII fold) → execute id (Vite overlay wins, **C542-5**).
5. Checksummed `terra1`.
6. Else `null`.

Outbound: `executeIdToQueryToken` → published casing (`cLUNC`, `vFDUSD`, `SpaceUSD`, `UST1`, `LUNC` / `USTC`). Uniswap names stay inbound-only.

## Canonical code

| File | Role |
|------|------|
| `scripts/qa/tokenlist_unique_symbols.py` | Static uniqueness CI (no network) |
| `frontend-dapp/src/utils/tokenlistQueryCatalog.ts` | Bundled maps + overlays; shared with Create Pair |
| `frontend-dapp/src/utils/swapQueryParams.ts` | Parse + `canonicalSwapSearch` symbols |
| `frontend-dapp/src/components/swap/SwapSharePairLabel.tsx` | Share chrome logos |
| `frontend-dapp/src/utils/ust1SecondaryMarket.ts` | `ust1SecondarySwapPath` → `/?from=UST1&to=vFDUSD` |

## Do / don’t

- **Do** bundle `tokenlist.json` (same relative import as Create Pair). Overlay env addresses for LocalTerra.
- **Do** fail the MR on colliding tickers. Prefer reject non-ASCII tickers.
- **Don’t** `fetch(tokenlist.json)`. **Don’t** LCD-fetch `token_info.symbol` for query ids.
- **Don’t** force-uppercase outbound (`cLUNC` must not become `CLUNC`).
- **Don’t** put pair logos on Trader / Portfolio Share.
- **Don’t** persist query in `localStorage`. **Don’t** map `0x` to Terra. **Don’t** treat `from=UST1` as `/ust1` mint (**U1**).
- **Don’t** honor `?showGems=1`. Production cannot encode or share a gem.

## Regression

```bash
make verify-issue-715
make verify-issue-711
make verify-issue-713
make verify-issue-542
make verify-issue-665
python3 scripts/check_chrome_nesting.py
```

Vitest: `tokenlistQueryCatalog.test.ts`, `swapQueryParams.test.ts`, `sharePageLink.test.ts`, `SwapPage.queryParams.test.tsx`, `ShareLinkButton.test.tsx`, `ust1SecondaryMarket.test.ts`, `tradeQueryResolve.test.ts`.  
Playwright (5 workers, when Vite + factory): `e2e/swap-tokenlist-symbols-715.spec.ts`.

## Related

- [`AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md`](./AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md) — inbound parse (**Q711**)
- [`AGENTS_FRONTEND_SWAP_URL_SYNC.md`](./AGENTS_FRONTEND_SWAP_URL_SYNC.md) — rewrite / reverse / Share mount (**Q713**)
- [`AGENTS_FRONTEND_SHARE_LINK.md`](./AGENTS_FRONTEND_SHARE_LINK.md) — TS-2 Swap exception; Trader stays text-only
- [`AGENTS_FRONTEND_CREATE_PAIR_PICKER.md`](./AGENTS_FRONTEND_CREATE_PAIR_PICKER.md) — same bundled list + overlays (**C542-5 / C542-11**)
- [`AGENTS_FRONTEND_NATIVE_TICKERS.md`](./AGENTS_FRONTEND_NATIVE_TICKERS.md) — display LUNC/USTC (#630)
- [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) — gems / **X8**
- [`AGENTS_UST1_SECONDARY_AMM.md`](./AGENTS_UST1_SECONDARY_AMM.md) — **U1**; `ust1SecondarySwapPath`
- [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) — no extra Share chrome
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — silent rewrite (#489)
