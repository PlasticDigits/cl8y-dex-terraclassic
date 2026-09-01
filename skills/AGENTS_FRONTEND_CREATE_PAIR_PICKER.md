# Agent playbook: Create Pair listed-CW20 picker (GitLab #542)

Audience: third-party agents touching `/create`, token pickers, or `tokenlist.json`.

**Issue:** [GitLab **#542**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/542)  
**Invariants:** [`docs/frontend.md` § Create pair listed CW20 picker](../docs/frontend.md#create-pair-token-picker) (**C542-1–C542-11**)  
**Related:** [#382](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/382) checksum, [#376](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/376) code-ID ≠ safety, [#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378) logo allowlist, [#481](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/481) Swap search (wrong universe), [#508](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/508) UST1 AMM ≠ mint, [#534](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534) economic-first sort, [#602](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/602) Create Token → `/create` copy-address (**P402-5**), [#713](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/713) `/create?a=&b=` prefill

## Problem class

`/create` used to be two raw `terra1…` fields. Operators creating cLUNC/UST1 or CL8Y/cUSTC had to paste 64-character addresses. Swap’s `TokenSearchSelect` universe is **factory-routable tokens** (pairs that already exist) — the opposite of Create Pair.

## Do / don’t

- **Do** build options with [`createPairTokenCatalog.ts`](../frontend-dapp/src/utils/createPairTokenCatalog.ts) (`getCreatePairCw20Options`). Bundle `tokenlist/tokenlist.json`; overlay `VITE_*` wrap / UST1 / CL8Y addresses via shared [`tokenlistQueryCatalog.ts`](../frontend-dapp/src/utils/tokenlistQueryCatalog.ts); append `SOFT_LAUNCH_MINTABLE_TOKENS` only when `retailExposeTestTokens()` **and** those env addresses are set ([#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562) **P562-5**). Duplicate tickers are a **CI** failure ([#715](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/715) **TL-1**), not a runtime skip.
- **Do** keep a **Custom contract** paste path. Paste still runs #382 format + checksum before submit.
- **Do** keep `useCodeIdCheck` on both listed and pasted addresses. Listed ≠ skip whitelist.
- **Do** compare A/B case-insensitively (`sameCreatePairAddress`).
- **Do** reuse `TokenSearchSelect` **as a control** with catalog ids (keyboard / 128-char cap / logo allowlist).
- **Don’t** import `getAllTokens(pairs)` or feed this catalog into Swap / Trade / Mint pickers (**C542-8**).
- **Don’t** fetch GitLab raw / CoinGecko / indexer `GET /api/v1/tokens` for this list (**C542-11**).
- **Don’t** offer native `uluna` / `uusd` / LUNC / USTC. `tokenAssetInfo` would encode them as `native_token`; factory rejects natives.
- **Don’t** use tokenlist `decimals` for any amount math. Create Pair sends no amounts.
- **Don’t** add “verified safe” / “governance-audited token” copy. Listed = in our published catalog only.
- **Do** honor checksummed `/create?a=&b=` (and `tokenA`/`token_a`) prefill ([#713](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/713) **Q713-6** / **C542-11**). Hostile / native / bad checksum ignored per side. **No** auto-submit. Swap deep links stay a **different** universe ([#711](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/711)).
- **Don’t** change factory `CreatePair`, whitelist, pair-creation fee, or LP ticker rules.

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/createPairTokenCatalog.ts` | Bundled CW20 catalog + env overlay + gem append + dedupe |
| `frontend-dapp/src/components/create/CreatePairTokenField.tsx` | Combobox + custom disclosure |
| `frontend-dapp/src/pages/CreatePairPage.tsx` | Submit / whitelist / UST1 notice |
| `tokenlist/tokenlist.json` | Published CW20 + native rows (natives dropped here) |

## Regression

```bash
make verify-issue-542
```

Vitest: `createPairTokenCatalog.test.ts`, `CreatePairPage.test.tsx`, `TokenSearchSelect.test.tsx` (Swap universe unchanged).  
Playwright (when `make has-localterra`): `e2e/create-pair-picker-542.spec.ts`, `e2e/create-pair.spec.ts`.

## Related

- [`AGENTS_FRONTEND_TOKEN_SEARCH.md`](./AGENTS_FRONTEND_TOKEN_SEARCH.md) — Swap combobox; factory gate only
- [`AGENTS_FRONTEND_NATIVE_TICKERS.md`](./AGENTS_FRONTEND_NATIVE_TICKERS.md) — Swap may label **LUNC** / **USTC**; Create Pair still excludes natives ([#630](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/630) / **C542-2**)
- [`AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](./AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) — economic-first sort (**P534-7**)
- [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) — production does not append gems (**P562-5**, [#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562))
- [`AGENTS_UST1_SECONDARY_AMM.md`](./AGENTS_UST1_SECONDARY_AMM.md) — Create Pair notice (**U1**)
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — short labels (#489)
- [`AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md`](./AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md) — Swap inbound parse (#711)
- [`AGENTS_FRONTEND_SWAP_URL_SYNC.md`](./AGENTS_FRONTEND_SWAP_URL_SYNC.md) — `/create?a=&b=` prefill (#713)
- [`AGENTS_FRONTEND_SWAP_TOKENLIST_SYMBOLS.md`](./AGENTS_FRONTEND_SWAP_TOKENLIST_SYMBOLS.md) — unique tokenlist symbols (#715)
