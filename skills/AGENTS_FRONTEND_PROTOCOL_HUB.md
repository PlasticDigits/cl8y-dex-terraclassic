# Agent playbook: Protocol DEX hub wrap identity + LUNC column (GitLab #570)

Audience: third-party agents touching `/protocol` DEX hub prices, `GET /api/v1/hub-prices`, or hub ticker allowlists.

**Issue:** [GitLab **#570**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/570)  
**Parent hub USD:** [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) (**H1–H16**)  
**Protocol layout:** [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) (**P550-1**)  
**Token identity / explorer:** [`AGENTS_FRONTEND_TOKEN_IDENTITY.md`](./AGENTS_FRONTEND_TOKEN_IDENTITY.md) (**T541-2**)  
**Verify:** `make verify-issue-570`

## Problem class

The Protocol **DEX hub prices** card showed cUSTC / UST1 / USTR. UST1 and USTR had **source pair** `AddressRow`s; cUSTC (the USD anchor) had no contract chrome because `source_pair` is always null. LUNC/USD existed only as a CEX tab, not in the compact hub strip.

## Invariants (H11–H16)

| ID | Rule |
|----|------|
| **H11** | Hub card columns are **cUSTC, LUNC, UST1, USTR** (`grid-cols-1 sm:grid-cols-2 xl:grid-cols-4`). The four cells stay in the DOM on hub 502 (USD `—`; wrap rows still come from env overlay). CEX tabs stay exactly `ustc` \| `lunc` \| `vfdusd`. Page order unchanged (**P550-1**). |
| **H12** | `usd(LUNC)` on this card = #515 LUNC CEX (`usd(cLUNC) = usd(uluna)`). Oracle down → `—`, not `$1` / `$0` / USTC. **Not** a DEX pool mark. Independent of USTC outage. |
| **H13** | Oracle-anchored columns (cUSTC, LUNC) render a wrap **CW20** `AddressRow` when the address passes `getExplorerAddressUrl`. cUSTC = `HUB_CUSTC_ADDRESS` / `VITE_USTC_C_TOKEN_ADDRESS`. LUNC = **cLUNC wrap** (`HUB_CLUNC_ADDRESS` / `VITE_LUNC_C_TOKEN_ADDRESS`). Native `uluna` is never given a Finder URL. |
| **H14** | UST1/USTR keep **source pair** rows (pair contracts, not tokens). cUSTC/LUNC `source_pair` stays null. Do not invent a pool for oracle wraps. |
| **H15** | `GET /api/v1/hub-prices` tickers `custc` \| `lunc` \| `ust1` \| `ustr`. `GET /hub-prices/lunc` is the DEX-card snapshot; `GET /oracle/price/lunc` stays CEX history. Unknown hub ticker / `clunc` path → **400**. `GET /oracle/price/custc\|ust1\|ustr` stay **400**. Additive JSON `asset_address` is the **configured** wrap, sanitized (`terra1` only). |
| **H16** | This skill + invariants + `make verify-issue-570`. Configured env overlay wins over API `asset_address`. Invalid / `javascript:` / empty → omit the token row. |

## Do / don’t

- **Do** keep one `getHubPrices()` on Protocol (LUNC is in that snapshot). Fold hub 502 into `detectMarketDataOutage`. Four hub cells stay painted on 502 (USD `—`; wrap rows from env); `RetryError` is additive.
- **Do** pass wrap addresses through `getExplorerAddressUrl` (never interpolate env or indexer JSON into `href`).
- **Do** use aria `Copy cUSTC token contract` / `Copy cLUNC wrap contract` — distinct from `Copy UST1 source pair`.
- **Don’t** add a fourth CEX tab or `getOraclePrice('ustr'|'ust1'|'custc')`.
- **Don’t** rank a factory pair to fill cUSTC/LUNC `source_pair`.
- **Don’t** match `symbol === 'cUSTC'` / `'LUNC'`. Identity is contract.
- **Don’t** retarget `volume_usd`, UST1/USTR ranking, Charts USD, `/ust1`, or portfolio P&L (LUNC P&L stays CEX per **P560-3**).
- **Don’t** clone hub AddressRows onto Swap / Trade / Charts (#541).

## Env

| Variable | Default |
|----------|---------|
| `HUB_CLUNC_ADDRESS` | Columbus-5 tokenlist cLUNC |
| `VITE_LUNC_C_TOKEN_ADDRESS` | dApp overlay for the LUNC column wrap |
| `VITE_USTC_C_TOKEN_ADDRESS` | dApp overlay for the cUSTC column wrap |

LocalTerra: set hub wraps to deployed CW20s in `indexer/.env` and `frontend-dapp/.env.local`.

## Related

- [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) — ranking H1–H10
- [`AGENTS_INDEXER_EXTERNAL_ORACLE.md`](./AGENTS_INDEXER_EXTERNAL_ORACLE.md) — CEX catalog stays 3 tickers
- [`AGENTS_FRONTEND_ADDRESS_ROW.md`](./AGENTS_FRONTEND_ADDRESS_ROW.md) — copy + explorer primitive
- [`AGENTS_FRONTEND_TOKEN_IDENTITY.md`](./AGENTS_FRONTEND_TOKEN_IDENTITY.md) — native copy-only
