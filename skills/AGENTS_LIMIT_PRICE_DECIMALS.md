# Agent playbook: Limit price decimals-normalized band (GitLab #529)

Audience: third-party agents placing or displaying **limit orders** on mixed-decimal pairs (UST1 6 / USTR 18), or touching `validate_limit_order_price`.

**Issue:** [GitLab **#529**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/529)  
**Invariants:** **L20** in [`docs/contracts-security-audit.md`](../docs/contracts-security-audit.md) (human-scale band); product [`docs/limit-orders.md` § Limit price band](../docs/limit-orders.md#limit-price-band-gitlab-467)

## Problem class

On-chain `price` is **raw** token1 base units per token0 base unit (`fill × price` / `1/price`). #467 bounded that raw `Decimal` to **[1e-9, 1e9]**. A 6-vs-18 pair at an ordinary ~79 human USTR/UST1 has raw `79 × 10^12 ≈ 7.9e13` — over the old max by ~79,000×. The reverse orientation (`≈ 1.3e-14`) undershoots the min. Placement reverted `Invalid hybrid parameters`. Pool swaps were unaffected.

## Invariants (L529-1–L529-5)

| ID | Rule |
|----|------|
| **L529-1** | Execution / book sort keep **raw** `price` (token1 units / token0 units). Do not rewrite stored prices to human. |
| **L529-2** | Placement, ladder expansion, and **`UpdateLimitOrderPrice`** apply **[`MIN_LIMIT_PRICE`, `MAX_LIMIT_PRICE`]** = **[1e-9, 1e9]** to **`human = raw × 10^(decimals0 − decimals1)`**. Pair queries live CW20 `token_info` decimals (not persisted on pair state). |
| **L529-3** | Equal-decimal pairs behave as before (#467): `Decimal::raw(1)` is still rejected. Match-time overflow **skip** on `1/price` / `fill × price` stays (**L20** belt). |
| **L529-4** | dApp UI, gates, refs, and crossing checks stay **human**. Convert **human → raw** only at the CW20/pair execute edge (`scaleHumanLimitPriceForChain`). Convert **raw → human** when reading indexer/LCD book or placement prices (`scaleRawLimitPriceForDisplay`). |
| **L529-5** | Indexer / LCD limit-book `price` strings remain the on-chain raw `Decimal`. Do not silently treat them as human on a 6/18 pair. |

## Do / don’t

- **Do** send raw `78.76e12` (or the dApp-scaled string) for UST1(6)/USTR(18) at human 78.76.
- **Do** keep #524 display invert as a **reciprocal** of the **human** factory price — invert after raw→human, never instead of it.
- **Don’t** widen the raw band to ±1e21 as the primary fix (reopens #467 and still fails 24-dec tokens).
- **Don’t** change match arithmetic to human prices.
- **Don’t** compare a typed human limit to an unscaled indexer book `price` on mismatched decimals (insert hints, crossing, ladder depth).

## Canonical code

| File | Role |
|------|------|
| [`limit_placement.rs`](../smartcontracts/packages/dex-common/src/limit_placement.rs) | `human_scale_limit_price`, `validate_limit_order_price(price, dec0, dec1)`, `expand_limit_ladder(..., dec0, dec1)` |
| [`asset_decimals.rs`](../smartcontracts/contracts/pair/src/asset_decimals.rs) | Live CW20 decimals for the pair |
| [`limitOrderPriceScale.ts`](../frontend-dapp/src/utils/limitOrderPriceScale.ts) | Client human↔raw |
| [`pair.ts`](../frontend-dapp/src/services/terraclassic/pair.ts) | Submit-edge scale |

## Regression

```bash
make verify-issue-529
make verify-issue-467
```

```bash
cd smartcontracts && cargo test -p dex-common validate_limit_price --quiet
cd smartcontracts && cargo test -p cl8y-dex-tests place_and_fill_limit_on_six_vs_eighteen_pair place_limit_on_eighteen_vs_six_pair --quiet
cd frontend-dapp && npm test -- --run limitOrderPriceScale pair.test
```

## Related

- [`AGENTS_BOOK_MATCH_HINT_SECURITY.md`](./AGENTS_BOOK_MATCH_HINT_SECURITY.md) — **L20** match skip
- [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md) — human refs / gates
- [`AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](./AGENTS_LIMIT_ORDER_BATCH_LADDER.md) — ladder expand still human in the preview; scale at submit
- [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) — invert is not decimal scale
- [`AGENTS_INDEXER_PAIR_PRICE_USD.md`](./AGENTS_INDEXER_PAIR_PRICE_USD.md) — swap tape human scale (#522); limit book prices stay raw
- [`AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md`](./AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md) — cancel does not use the price band
