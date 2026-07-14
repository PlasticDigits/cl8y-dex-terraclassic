# Agent playbook: anti-cognitive-overload retail copy

Use when adding or editing **user-visible** strings on the dApp ([GitLab #488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/488) reopen).

## Invariants

1. **Audience = humans.** Agents read `docs/` and `skills/` — do not paste protocol essays onto cards.
2. **Labels** ≤ ~5 words (`Pay`, `Receive`, `When 1 LUNC is worth`).
3. **Blocking errors** ≤ 1 short sentence + optional **Docs** link.
4. **Never** show `token0`, `token1`, or raw `bid`/`ask` in retail UI — use token **symbols** and **Buy** / **Sell**.
5. **Keep** safety gates, risk acknowledgement (`NFA_SHORT`), and accurate error *conditions* — only shorten wording.
6. Prefer progressive disclosure (`<details>Signing details</details>`) over always-on grids.

## QuickSwap-aligned Limits IA

Rate (“When 1 {base} is worth”) → % chips → Pay → flip → Receive → Expiry → CTA. Book and open orders **below** the place card.

## Canonical docs

| Doc | Role |
|-----|------|
| [`docs/design-system.md`](../docs/design-system.md) | Visual + copy principles |
| [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md) | Token/chrome + copy rules |
| [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md) | Limit place-card order |

## Shared copy modules

Short leads live in `marketDataServiceCopy.ts`, `indexerTradeOutageCopy.ts`, `lcdConnectivity.ts`, `feeDiscountRegistryWarning.ts`, `blacklist.ts` (`describeTradingBlacklistBlock`).
