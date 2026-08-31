# Agent playbook: native LUNC / USTC picker labels

Audience: third-party agents touching token pickers, `useTokenDisplayInfo`, indexer native `assets` upsert, or wrap selector copy.

**Issue:** [GitLab **#630**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/630)  
**Invariants:** [`docs/frontend.md` § Token search](../docs/frontend.md#token-search-combobox) (**N630-1–N630-8**)  
**Related:** [#507](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/507) wrap product symbols, [#541](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/541) copy payload stays denom, [#481](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/481) factory gate, [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/489) no essays, [#542](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/542) Create Pair excludes natives.

## Problem class

Retail pickers must show **LUNC** / **USTC**, never bank denoms `uluna` / `uusd`. Wrap CW20s stay **cLUNC** / **cUSTC**. Ids used for balances, quotes, and execute stay `uluna` / `uusd`. A stale indexer `symbol=uluna` (or a spoofed ticker) must not win over the static registry.

## Do / don’t

- **Do** resolve visible tickers with [`registryProductSymbol`](../frontend-dapp/src/utils/tokenRegistry.ts) **before** indexer / `token_info` text. Shared by `getTokenDisplaySymbol` and `useTokenDisplayInfo`.
- **Do** keep four distinct rows when wrap env is set: LUNC, cLUNC, USTC, cUSTC. Logos stay LUNC.png / CLUNC.png / USTC.png / CUSTC.png.
- **Do** keep search haystack with both denom and product ticker (`uluna` and `LUNC`).
- **Do** copy the **denom** (`uluna` / `uusd`), never the display ticker ([#541](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/541)).
- **Don’t** rename execute / React Query / wrap-mapper ids.
- **Don’t** invent tickers for unknown bank / IBC denoms — fail closed as the raw denom.
- **Don’t** add natives to Create Pair ([#542](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/542)).
- **Don’t** ship an always-on “uluna means LUNC” blurb ([#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/489)).
- **Don’t** treat a spoofed indexer symbol as gem/hub identity ([#534](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534) / [#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562)).

## Invariants

| ID | Meaning |
|----|---------|
| **N630-1** | Visible picker / `TokenDisplay` / `TokenIdentity` text for `uluna` / `uusd` is **LUNC** / **USTC**. |
| **N630-2** | Wrap rows stay **cLUNC** / **cUSTC** even when indexer/on-chain say `LUNC-C` / `USTC-C` (**#507**). |
| **N630-3** | Token **id** in `onChange`, `data-testid`, balances, and execute remains `uluna` / `uusd` (or wrap CW20 address). |
| **N630-4** | Registry allowlist wins for known natives/wraps. Indexer `symbol=uluna` / `UST1` / HTML cannot override. |
| **N630-5** | Unknown natives (`ufoo`, `ibc/…`) display as the raw denom (or indexer symbol if present and not a known-native override). |
| **N630-6** | Search matches `LUNC`, `lunc`, `uluna`, `USTC`, `uusd`. Haystack keeps both forms. |
| **N630-7** | Copy payload is the denom; natives stay copy-only (no Finder URL). |
| **N630-8** | Indexer first-insert + repair write `LUNC` / `USTC` for `denom=uluna` / `uusd` only. Other denoms stay denom/denom. Wrap CW20 rows untouched. |

## Canonical code

| File | Role |
|------|------|
| `frontend-dapp/src/utils/tokenRegistry.ts` | `DENOM_MAP` + `registryProductSymbol` |
| `frontend-dapp/src/utils/tokenDisplay.ts` | `getTokenDisplaySymbol` uses the helper first |
| `frontend-dapp/src/hooks/useTokenDisplayInfo.ts` | Shared label source for Swap / Pool / Pay / Mint / TokenDisplay / TokenIdentity |
| `indexer/src/indexer/asset_resolver.rs` | Native upsert labels + denom-as-symbol repair |
| `indexer/migrations/20260825140000_repair_native_bank_tickers.sql` | One-time catalog repair |

## Regression

```bash
make verify-issue-630
```

Vitest: `tokenDisplay.test.ts`, `tokenRegistry.test.ts`, `tokenSearchQuery.test.ts`, `useTokenDisplayInfo.test.tsx`, `TokenSearchSelect.issue630.test.tsx`, `tokenIdentity.test.ts`. Indexer lib: `cargo test --lib asset_resolver`.

## Related

- [`AGENTS_FRONTEND_TOKEN_SEARCH.md`](./AGENTS_FRONTEND_TOKEN_SEARCH.md) — Swap combobox / factory gate
- [`AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md`](./AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md) — query tickers resolve to `uluna`/`uusd`; UI still shows LUNC/USTC ([#711](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/711))
- [`AGENTS_FRONTEND_TOKEN_IDENTITY.md`](./AGENTS_FRONTEND_TOKEN_IDENTITY.md) — copy/explorer payloads
- [`AGENTS_MAINNET_WRAP_ENABLEMENT.md`](./AGENTS_MAINNET_WRAP_ENABLEMENT.md) — wrap env + cLUNC/cUSTC
- [`AGENTS_FRONTEND_CREATE_PAIR_PICKER.md`](./AGENTS_FRONTEND_CREATE_PAIR_PICKER.md) — natives never selectable
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — no denom essays
- [`AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](./AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) — `uluna` stays economic
- [`AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md`](./AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md) — Manage provide labels use LUNC/cLUNC not Asset A (#661)
