# Agent playbook: Human tape / wallet amounts (GitLab #557)

Audience: third-party agents touching Charts / Trade / Trader / Portfolio **Recent trades**, wallet pair history, or indexer `TradeResponse` / `LimitFillResponse` JSON/CSV.

**Issue:** [GitLab **#557**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/557)  
**Invariants:** [`docs/frontend.md`](../docs/frontend.md) § Tape amounts (human scale) (**T557-1–T557-11**); [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) **Trade tape decimals #557**  
**Related:** [#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522) human tape **Price** (not USD), [#524](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524) display invert, [#479](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/479) wallet history columns, [#551](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/551) portfolio P&amp;L (out of scope), [#548](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/548) / [#553](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/553) volume USD (out of scope)

## Problem class

Indexer `offer_amount` / `return_amount` (and fill `token0_amount` / `token1_amount`) are **raw chain integers**. The dApp passed them to `formatNum`, so a 1 UST1 swap printed **`1.000M`**. Mixed 6/18 pairs compact as **`T`**. Tape **Price** after #522 is already human quote-per-base and must stay that way.

## Invariants (T557-1–T557-11)

| ID | Rule |
|----|------|
| **T557-1** | Amount in / out are **human** token amounts via `formatTokenAmount(raw, decimals)` (API `offer_decimals` / `ask_decimals`, else matching `activePair` legs). Never `formatNum(raw)`. |
| **T557-2** | 18-dec amounts compact as `T` only when the **human** size is ≥ 1e12. Ordinary USTR fills are `10`, not `10.00E` / `T`. |
| **T557-3** | Charts and Trade share [`TradesTable`](../frontend-dapp/src/components/ui/TradesTable.tsx). Do not fork a Charts-only formatter. |
| **T557-4** | Trader / Portfolio mixed history uses **per-row** API decimals. Never apply one pair’s decimals to every row. |
| **T557-5** | Wallet pair history Amount in/out/Price and fill sizes use the same helpers. CSV download stays **raw** indexer file. |
| **T557-6** | Tape **Price** is human quote-per-base (`formatPairPrice`). Never USD. Never compact `T` from raw 18/6. |
| **T557-7** | When Charts/Trade invert is on, Price is the reciprocal of **human** `price` (`invertFinitePositive`). Amount in/out stay offer → ask (not swapped). |
| **T557-8** | Integrator JSON/CSV **raw** amount columns stay **plain integer digit strings** (`bd_plain_string`, never `1e+19` — dApp `BigInt` would show `—`). Additive `offer_decimals` / `ask_decimals` / `token0_decimals` / `token1_decimals` only. |
| **T557-9** | Missing / out-of-range decimals (`<0` or `>38`) → UI `—`. Zero amounts with known decimals → `0`. No `NaN` / `Infinity`. |
| **T557-10** | After humanizing, amount cells include the pay/receive **symbol** (Pair column alone is not enough). |
| **T557-11** | Green/red buy color follows **display-base**: paying display-quote is a buy. Paying factory `asset_0` while inverted is a buy of display-base. Amounts still offer → ask. |

## Do / don’t

- **Do** take decimals from indexed `assets` rows (asset id), never from wasm events or `offer_asset` symbol matching across pairs.
- **Do** clamp decimals to `0..=38` before publishing or rendering.
- **Do** pass `inverted={pairOrientation.inverted}` from Charts and Trade only.
- **Don’t** `formatNum(t.offer_amount)` / `formatNum(t.return_amount)` on the tape.
- **Don’t** invert USD (`1/price_usd`) into the Price column.
- **Do** serialize tape/CSV amounts with `bd_plain_string` (`to_plain_string`), never `BigDecimal::to_string` / scientific notation.
- **Don’t** rewrite CSV `offer_amount` / `return_amount` to human.
- **Don’t** use tape amounts for settlement.

## Canonical code

| File | Role |
|------|------|
| [`indexer/src/api/pairs.rs`](../indexer/src/api/pairs.rs) | `api_asset_decimals`, `offer_decimals` / `ask_decimals` on `TradeResponse`, fill token decimals |
| [`indexer/src/api/text_csv.rs`](../indexer/src/api/text_csv.rs) | Additive CSV columns at end of header |
| [`frontend-dapp/src/utils/tradeTapeDisplay.ts`](../frontend-dapp/src/utils/tradeTapeDisplay.ts) | Shared scale / invert / buy-color helpers |
| [`frontend-dapp/src/components/ui/TradesTable.tsx`](../frontend-dapp/src/components/ui/TradesTable.tsx) | Shared tape |
| [`frontend-dapp/src/components/trade/WalletIndexerHistoryPanel.tsx`](../frontend-dapp/src/components/trade/WalletIndexerHistoryPanel.tsx) | Pair wallet history |

## Regression

```bash
make verify-issue-557
```

## Related

- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — tape Price is human; Price (USD) is `price_usd`
- [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) — invert is display-only; tape amounts stay offer→ask
- [`AGENTS_FRONTEND_ORDER_HISTORY.md`](./AGENTS_FRONTEND_ORDER_HISTORY.md) — wallet history columns; display human, CSV raw
- [`AGENTS_FRONTEND_PORTFOLIO_PNL.md`](./AGENTS_FRONTEND_PORTFOLIO_PNL.md) — P&amp;L scale is #551, not this issue
- [`AGENTS_POST_MERGE_STACK.md`](./AGENTS_POST_MERGE_STACK.md) — Coolify frontend + indexer restart with this cut ([#573](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/573))
