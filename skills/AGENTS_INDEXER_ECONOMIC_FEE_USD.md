# Agent playbook: factory economic fee USD (CL8Y + listed non-gems) (GitLab #683)

Audience: third-party agents touching protocol treasury `fee_usd`, `/protocol` fee headlines, or DeFiLlama daily fees.

**Issue:** [GitLab **#683**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/683)  
**Invariants:** [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) (row **Economic fee USD #683**)  
**Verify:** `make verify-issue-683` · keep `make verify-issue-586` · `613` · `614` · `556` · `569` · `631` green

## Problem class

AMM / book commission is **ask-denominated**. Buying CL8Y with UST1 stores the fee **in CL8Y**. P522-Q only prices USTC/cUSTC/LUNC/cLUNC/UST1/USTR, so `quote_usd_kind("CL8Y")` is `None`, `fee_usd` stays NULL, and 24h/7d/30d `SUM`s drop those rows. Human amount (~1.298 CL8Y) was visible; USD was an em-dash.

## Invariants (EFee-1–EFee-8)

| ID | Rule |
|----|------|
| **EFee-1** | Fee USD lookup: P522-Q / hub identity → factory economic mark by **CW20 contract** → `None`. Never `symbol=CL8Y` / `CL8Y-cb` / `TCL8Y` on a spoof native. Pin `HUB_CL8Y_ADDRESS` (columbus-5 default). |
| **EFee-2** | After hub bootstrap, price each factory-listed **economic** CW20 from the largest USD-TVL pair vs an **already-priced** hub/catalog leg (same ranking as USTR: humanized TVL, `HUB_USD_TVL_FLOOR`, stale/dust/same-asset skip). Single hop only — no circular unpriced↔unpriced walk. Never `$1` CL8Y. Never CoinGecko / CEX / Venus. |
| **EFee-3** | Exclude #562 gems (**addr + ticker**). Exclude Terra vFDUSD / FDUSD (X4 / #580). Hub wraps stay on P522-Q. Community-tax CW20s are economic only if factory-listed and not in the gem set. |
| **EFee-4** | Persist marks in sibling table `economic_token_marks` — **not** GET `/hub-prices`. Hub card stays **four** cells (**H11**). Extra `hub_prices.ticker` (`cl8y`, `javascript:`) still **400**. |
| **EFee-5** | Stamp `fee_usd` at ingest. **NULL-only** backfill after hub refresh (as-of, not historical MTM). Never `UPDATE` a non-null stamp (**C568-1**). |
| **EFee-6** | Partial windows stay priced SUM. Idle → `"0"`. All activity unpriced → `null` / `—`. GET `/overview` and `/protocol/fees` stay O(1) rollup / 60s cache (**PFee-8**). Llama headline `daily_fees_usd` uses the same partial SUM ([#687](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/687)); gem-pair exclude unchanged. |
| **EFee-7** | Volume / TVL stay P522-Q + hub. This is **fee-token USD**, not a second volume catalog and not a fifth hub column. |
| **EFee-8** | Verify: `make verify-issue-683`. Related: `586` / `613` / `614` / `556` / `569` / `631`. |

## Env

| Variable | Default |
|----------|---------|
| `HUB_CL8Y_ADDRESS` | Columbus-5 CL8Y CW20 (`DEFAULT_HUB_CL8Y_ADDRESS`) |
| `HUB_USD_TVL_FLOOR` | `100` (same as hub ranking) |

LocalTerra: set `HUB_CL8Y_ADDRESS` to the deployed TCL8Y CW20 (`VITE_CL8Y_TOKEN_ADDRESS`). Coolify pin is the **indexer** env, not Vite-only.

## Do / don’t

- **Do** key identity by contract/denom (A1). Display ticker **CL8Y-cb** is a wallet label for the pinned columbus-5 CW20 unless a different contract is proven.
- **Do** walk factory `pairs` only (P1). Dust / stale / below-floor pairs mint no mark.
- **Don’t** add a CEX `cl8y` tab or poll `ceramicliberty-com`.
- **Don’t** rewrite historical non-null `fee_usd` when the live mark jumps.
- **Don’t** `SUM protocol_fee_events` on GET.
- **Don’t** price vFDUSD from CEX FDUSD or Venus `exchangeRateCurrent`.

## Related

- [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) — **PFee-7** now includes factory economic marks
- [`AGENTS_INDEXER_HUB_USD.md`](./AGENTS_INDEXER_HUB_USD.md) — hub card stays four cells; internal marks are sibling
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — P522-Q catalog unchanged (`CL8Y` symbol still `None`)
- [`AGENTS_DEFILLAMA.md`](./AGENTS_DEFILLAMA.md) — daily fees reuse stamped `fee_usd`; headline is partial SUM ([#687](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/687))
- [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) — gem addr + ticker (#562)
- Post-merge leftover: [#686](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/686) / `make verify-issue-686` / [`AGENTS_POST_MERGE_OPS_686.md`](./AGENTS_POST_MERGE_OPS_686.md)
