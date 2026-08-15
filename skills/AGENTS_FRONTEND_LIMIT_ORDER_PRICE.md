# Agent playbook: Limit order price field (trade + standalone page)

Use when changing **limit price** UX on `/trade` or `/limits`: reference line from tape **or AMM pool**, % deviation, headline-scaled USD, submit validation, **escrow headline USD** ([`escrowAmountUsdAnchorNotional`](../frontend-dapp/src/utils/limitOrderPriceReference.ts); [GitLab **#155**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/155)), or the **When 1 {token} is worth** rate label + **side-aware** % chips ([GitLab **#495**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/495)).

**#488 IA (reopen):** place-card order is **rate** (“When 1 {token0} is worth”) → **% chips** → **Pay** → **Receive** (read-only expected fill) → **Expiry**. Order book + open placements sit **below** the place card. No instructional paragraphs; blocking invalid-direction / escrow / gas errors stay visible. See [`docs/design-system.md`](../docs/design-system.md) § Limit place IA.

**#495 chip invariant:** magnitudes stay `[0, 1, 5, 10]`, but applied deviation is **signed by side** via `signedLimitPriceDeviationPercent` / `limitPriceFromRefDeviationChip`:
- **Bid (buy):** below ref — labels `0%−`, `−1%`, `−5%`, `−10%`
- **Ask (sell):** above ref — labels `0%+`, `+1%`, `+5%`, `+10%`
- Magnitude **`0`** never means exact at-ref (equality is invalid per #154); it maps to ±`LIMIT_PRICE_NEAR_MARKET_DEVIATION_PERCENT` (0.01%).
Do **not** call `limitPriceFromRefDeviationPercent(ref, unsignedPreset)` from chip clicks without signing for side.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade page — limit order price field](../docs/frontend.md#trade-page-limit-order-price) | Invariants (reference, pool fallback, deviation, USD anchor, submit gate, side-aware rate chips) — [GitLab **#154**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/154), [**#166**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/166), [**#495**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/495) |
| [docs/frontend.md § Limit place — escrow amount](../docs/frontend.md#limit-place-escrow-amount) | Escrow **Amount** headline USD + Bid/Ask amount reset / MAX re-apply — [GitLab **#155**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/155) |
| [docs/frontend.md § Trade page — limit order pre-submit summary](../docs/frontend.md#trade-page-limit-order-pre-submit-summary) | Resting-order copy, deviation recap, maker placement bps, min LUNC for place sequence — [GitLab **#157**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/157) |
| [docs/limit-orders.md § dApp: retail form](../docs/limit-orders.md#dapp-retail-form-wires-invariants) | Cross-link to #154 / #166 / #495 bullet, **#157** pre-submit summary bullet, and pure helpers list |
| [`limitOrderPriceReference.ts`](../frontend-dapp/src/utils/limitOrderPriceReference.ts) | `tradeToToken1PerToken0Human`, `resolveLimitOrderPriceRef`, `poolReservesToToken1PerToken0Human`, `pairDecimalsForLimitPriceRef`, deviation %, `anchorUsdForLimitPrice`, **`escrowAmountUsdAnchorNotional`** (#155), `LIMIT_PRICE_DEVIATION_CHIP_PRESETS`, `signedLimitPriceDeviationPercent` / `limitPriceFromRefDeviationChip` (#495), `limitPriceFromRefDeviationPercent`, direction checks |
| [`limitOrderExpectedReceive.ts`](../frontend-dapp/src/utils/limitOrderExpectedReceive.ts) | Full-fill expected receive after maker placement fee (#488 Receive row) |
| [`LimitOrderReceiveField.tsx`](../frontend-dapp/src/components/trade/LimitOrderReceiveField.tsx) | Read-only Receive row on `/limits` + `/trade` limit tab |
| [`limitOrderFeeSummary.ts`](../frontend-dapp/src/utils/limitOrderFeeSummary.ts) | `effectiveSwapFeeBps`, `makerPlacementFeeBps` (integer match to pair discount + `floor(effective/2)` placement leg — #157) |
| [`useLimitOrderMakerFeeRates.ts`](../frontend-dapp/src/hooks/useLimitOrderMakerFeeRates.ts) | React Query: `getPairFeeConfig` + `getTraderDiscount` for effective / maker placement bps in the pre-submit card |
| [`LimitOrderPreSubmitSummary.tsx`](../frontend-dapp/src/components/trade/LimitOrderPreSubmitSummary.tsx) | Pre-sign card: labeled action/pair/side/amount/chain (#461 / SEC-I05), resting vs taker semantics, deviation, maker fee, est. network fee |
| [`useLimitOrderPriceRefBundle.ts`](../frontend-dapp/src/hooks/useLimitOrderPriceRefBundle.ts) | React Query: tape first, then LCD `getPool` when tape missing; chain `token_info` decimals when registry lacks local CW20s (#166); exposes `refResolutionLoading` / `refResolutionError` for the place gate |
| [`limitOrderPricePlaceGate.ts`](../frontend-dapp/src/utils/limitOrderPricePlaceGate.ts) | `evaluateLimitOrderPricePlaceGate(side, price, ref, ctx?)` — mirrors submit button + mutation throw; **blocks** positive limits when ref unavailable (#166) |
| [`LimitOrderPriceField.tsx`](../frontend-dapp/src/components/trade/LimitOrderPriceField.tsx) | `LimitOrderPriceInputWithContext` (ref + side-aware % chips + `LimitOrderSideFlipButton`); no on-card instructional heading |
| [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) + [`TradePage.tsx`](../frontend-dapp/src/pages/TradePage.tsx) | Pass `indexerPair`, `latestTrade`, `tapeHeadlineUsd`; ticket runs `useLimitOrderPriceRefBundle` |
| [`LimitOrdersPage.tsx`](../frontend-dapp/src/pages/LimitOrdersPage.tsx) | Same hook + local `getPair` / `getTrades` queries |

## Rules of thumb

1. **Keep reference math in `limitOrderPriceReference.ts`** — UI components should not re-derive BigInt ratios inline.
2. **Tape headline string** passed to `anchorUsdForLimitPrice` must stay aligned with `PriceChart`’s `tapeLastPriceUsd`. Use `resolveTapeLastPriceUsd` (`trades[0].price_usd` or human price × quote catalog) — **never** raw `trades[0].price` ([#522](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/522)). Pool-only refs may leave headline USD as **—** until tape returns. The **escrow amount** USD line ([**#155**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/155)) consumes the **same** `tapeHeadlineUsd` + `refToken1PerToken0` tuple — do not fork a second oracle.
3. When changing submit rules, update **both** `evaluateLimitOrderPricePlaceGate` and the `placeMutation` throw path, plus Vitest under `utils/__tests__/limitOrderPrice*.test.ts`.
4. If copy or thresholds for “extreme deviation” change, update `docs/frontend.md` and this skill together.
5. **#166 invariant:** never allow a **positive** typed limit to submit without a resolved reference (tape or pool), unless product explicitly changes that contract.
6. **#157 / #488 pre-submit card:** keep compact labeled rows + fee math in [`LimitOrderPreSubmitSummary.tsx`](../frontend-dapp/src/components/trade/LimitOrderPreSubmitSummary.tsx) + [`limitOrderFeeSummary.ts`](../frontend-dapp/src/utils/limitOrderFeeSummary.ts); wire fee queries through [`useLimitOrderMakerFeeRates.ts`](../frontend-dapp/src/hooks/useLimitOrderMakerFeeRates.ts). **#461 / SEC-I05:** the card must include labeled **Action**, **Pair**, **Side**, **Pay/Amount**, and **Chain** rows (same `getNetworkBadgeCopy().fullLabel` as swaps) before the wallet opens — minimize instructional paragraphs; a single **Docs** link is enough. If copy or bps formula changes, update [`docs/frontend.md` § pre-submit summary](../docs/frontend.md#trade-page-limit-order-pre-submit-summary) and Vitest for `limitOrderFeeSummary` / `LimitOrderPreSubmitSummary`.
7. **Design system:** blue+gold tokens; gold = border/text only — [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md) ([#488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/488)).
8. **#488 place-card order:** do not put OrderBook / My placements above the place form on `/limits`; keep rate → chips → Pay → Receive → Expiry.
9. **#495 chips:** keep chip math in `limitOrderPriceReference.ts` (`signedLimitPriceDeviationPercent`, `limitPriceFromRefDeviationChip`, side-aware `matchingLimitPriceDeviationChip`). Bid chips must stay strictly below ref; ask chips strictly above. Update Vitest in `limitOrderPriceReference.test.ts` + `LimitOrderPriceField.test.tsx` and [`docs/frontend.md` § limit order price](../docs/frontend.md#trade-page-limit-order-price) when changing epsilon or labels.

## Related

- Anti-cognitive-overload retail copy: [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) ([#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489))
- Limit **Bid / Ask** side control (button radiogroup): [`AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md`](./AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md) ([GitLab **#153**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/153))
- Trade workspace layout: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Parked / expired limits: [`AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md`](./AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md)
- Placement gas presets (Advanced book walk): [`AGENTS_FRONTEND_LIMIT_ORDER_PLACEMENT_GAS.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PLACEMENT_GAS.md) ([GitLab **#204**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/204))
- Price chart / headline: [`AGENTS_FRONTEND_PRICE_CHART.md`](./AGENTS_FRONTEND_PRICE_CHART.md)
