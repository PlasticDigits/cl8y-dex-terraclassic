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

## QuickSwap-aligned Limits IA

Rate (“When 1 {base} is worth”) → % chips → Pay → flip → Receive → Expiry → CTA. Book and open orders **below** the place card.

## Canonical docs

| Doc | Role |
|-----|------|
| [`docs/design-system.md`](../docs/design-system.md) § Terminology glossary | Shared retail term definitions |
| [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md) | Token/chrome + copy rules |
| [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md) | Limit place-card order |
| [`AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md`](./AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md) | Buy/Sell {base} on `/trade` + `/limits` |
| [`AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md`](./AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md) | Onboarding strip + progressive disclosure |
| [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md) | Single Route row; Swap vs Best Trade disambiguation |
| [`AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](./AGENTS_FRONTEND_RISK_DISCLAIMERS.md) | Safety > brevity — keep required ack/footer |
| [docs/frontend.md § Retail copy & cognitive load](../docs/frontend.md#retail-copy-cognitive-load) | Engineering invariants + cross-links |

## Shared copy modules

Short leads live in `marketDataServiceCopy.ts`, `indexerTradeOutageCopy.ts`, `lcdConnectivity.ts`, `feeDiscountRegistryWarning.ts`, `blacklist.ts` (`describeTradingBlacklistBlock`).
