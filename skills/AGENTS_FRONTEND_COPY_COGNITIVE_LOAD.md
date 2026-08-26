# Agent playbook: anti-cognitive-overload retail copy

Use when adding or editing **user-visible** strings on the dApp ([GitLab #488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/488) reopen, [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489) docs/skills alignment).

## Invariants

1. **Audience = humans.** Agents read `docs/` and `skills/` — do not paste protocol essays onto cards.
2. **Labels** ≤ ~5 words (`Pay`, `Receive`, `When 1 LUNC is worth`).
3. **Blocking errors** ≤ 1 short sentence + optional **Docs** link.
4. **Never** show `token0`, `token1`, or raw `bid`/`ask` in retail UI — use token **symbols** and **Buy** / **Sell**.
5. **Keep** safety gates, risk acknowledgement (`NFA_SHORT`), and accurate error *conditions* — only shorten wording.
6. Prefer progressive disclosure (`<details>Signing details</details>`) over always-on grids.
7. **Docs link never replaces blocking errors** — optional depth only; do not remove risk ack, required footer, or trust-boundary warnings to save space.
8. **Silence over instructional fluff** — if Settings already enable a feature, do not add Execution copy telling the user to enable it. Example: hybrid on + empty Swap book leg → no “add a book leg” notice ([GitLab **#492**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/492), [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md)).
9. **No always-on educational / cross-nav / fee-trivia banners** on feature pages. **Do not ship or merge** copy that:
   - Explains architecture the nav already implies (“via the treasury wrap-mapper”, “this is not an AMM swap”, “oracle mint/redeem”).
   - Points users to other routes they can already find in nav (“For market trading… use Swap”, “UST1 is on UST1”).
   - Restates universal chain facts under every CTA (“You pay network gas…”, “Burn tax may apply…”) unless that fact is a **live blocking gate** for the current action.
   - Surfaces **dev/ops status** as retail chrome (“Mapper Ready”, contract addresses, “Unavailable” health rows) when pause/config failures are already enforced via CTA disable + short error.
   Page chrome = **title + controls + live status (fee/limit/pause) + CTA**. Put depth in docs/`<details>`, not permanent paragraphs. Bad example that was removed from `/wrap`: subtitle + Swap/UST1 blurb + Mapper Ready + gas/burn-tax footer.

## QuickSwap-aligned Limits IA

Rate (“When 1 {base} is worth”) → % chips → Pay → flip → Receive → Expiry → CTA. Book and open orders **below** the place card.

## Canonical docs

| Doc | Role |
|-----|------|
| [`docs/design-system.md`](../docs/design-system.md) § Terminology glossary | Shared retail term definitions |
| [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md) | Token/chrome + copy rules |
| [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) | Nested cards are visual noise (#653) — metric grids stay flat |
| [`AGENTS_FRONTEND_SWAP_DIRECTION_SEAM.md`](./AGENTS_FRONTEND_SWAP_DIRECTION_SEAM.md) | No lecture under the Swap flip; paint-only seam plate (#659) |
| [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md) | Limit place-card order |
| [`AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md`](./AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md) | Buy/Sell {base} on `/trade` + `/limits` |
| [`AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md`](./AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md) | Onboarding strip + progressive disclosure |
| [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) | Single Route row; Swap vs Best Trade disambiguation |
| [`AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](./AGENTS_FRONTEND_RISK_DISCLAIMERS.md) | Safety > brevity — keep required ack/footer |
| [`AGENTS_FRONTEND_PRODUCT_LINKS.md`](./AGENTS_FRONTEND_PRODUCT_LINKS.md) | Footer **Homepage** / **Bridge** only — not Swap lecture banners ([#663](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/663)) |
| [docs/frontend.md § Retail copy & cognitive load](../docs/frontend.md#retail-copy-cognitive-load) | Engineering invariants + cross-links |
| [`AGENTS_FRONTEND_POOL_TABLE.md`](./AGENTS_FRONTEND_POOL_TABLE.md) | `/pool` table; no header lectures ([#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547)) |
| [`AGENTS_FRONTEND_POOL_LP_HOWTO.md`](./AGENTS_FRONTEND_POOL_LP_HOWTO.md) | Opt-in `/pool` LUNC LP how-to (**H531-1–H531-10**, [#531](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/531)) — do not paste `frontend.md` pool math onto the page |
| [`AGENTS_FRONTEND_POOL_ONE_SIDED.md`](./AGENTS_FRONTEND_POOL_ONE_SIDED.md) | Retail `/pool` one-sided add/withdraw (**Z533-1–Z533-10**, [#533](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/533)) — option-less Token/Pair/Amount; wrap implied by the token |
| [`AGENTS_FRONTEND_POOL_ZAP_FLOORS.md`](./AGENTS_FRONTEND_POOL_ZAP_FLOORS.md) | One-sided zap execution floors (**Z559-1–Z559-4**, [#559](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/559)) — pre-sign min-swap is human units, not raw uints |
| [`AGENTS_FRONTEND_CREATE_PAIR_PICKER.md`](./AGENTS_FRONTEND_CREATE_PAIR_PICKER.md) | `/create` Token A/B + Custom contract; no “verified safe” banner ([#542](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/542)) |
| [`AGENTS_FRONTEND_TOKEN_IDENTITY.md`](./AGENTS_FRONTEND_TOKEN_IDENTITY.md) | Compact copy + explorer on Pool / Trade / Charts (**T541-1–T541-8**, [#541](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/541)) — icon controls, not address essays |
| [`AGENTS_FRONTEND_NATIVE_TICKERS.md`](./AGENTS_FRONTEND_NATIVE_TICKERS.md) | Picker tickers **LUNC** / **USTC** (**N630-1–N630-8**, [#630](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/630)) — no always-on “uluna means LUNC” essay |
| [`AGENTS_FRONTEND_TIERS_PHONE.md`](./AGENTS_FRONTEND_TIERS_PHONE.md) | `/tiers` phone cards (**T651-1–T651-8**, [#651](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/651)) — keep **Hold {n} CL8Y**; no fee-trivia banner |
| [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) | `/protocol` USD stats + one oracle card (**P550-1–P550-12**, [#550](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/550)) — short reference labels; no TWAP vs CEX essays |
| [`AGENTS_FRONTEND_TRAILING_WINDOW.md`](./AGENTS_FRONTEND_TRAILING_WINDOW.md) | Charts/Protocol/Pool **24h volume** is trailing `now − 24h`, not midnight reset (**W1–W5**, [#576](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/576)) |
| [`AGENTS_COMMUNITY_TAX_ROUTER.md`](./AGENTS_COMMUNITY_TAX_ROUTER.md) | Community tax sell/buy hints on every listed-pair swap (**C593-14**, [#607](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/607)) — short hints only, no router-architecture essay |

## Shared copy modules

Short leads live in `marketDataServiceCopy.ts`, `indexerTradeOutageCopy.ts`, `lcdConnectivity.ts`, `feeDiscountRegistryWarning.ts`, `blacklist.ts` (`describeTradingBlacklistBlock`), `trailingWindowCopy.ts` (Charts/Protocol/Pool 24h window, [#576](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/576)).
