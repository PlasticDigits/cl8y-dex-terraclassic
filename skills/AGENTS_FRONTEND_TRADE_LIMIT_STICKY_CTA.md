# Agent playbook: Trade limit ticket CTA opacity / guards

Use when changing the **`/trade` order ticket** money-CTA chrome opacity, or inline place/price/gas guard placement relative to that footer ([GitLab **#500**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/500)).

The CTA itself is a **ticket footer** (flex `shrink-0` sibling of the scrollport), not `position: sticky`. Dock / Chrome mid-form float: [`AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md`](./AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md) ([#527](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/527)). Do not add `position: sticky` / `fixed` to “fix” a float.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade page — ticket footer CTA](../docs/frontend.md#trade-page-ticket-footer-cta) | **T527-1–T527-10** plus `#500` opacity / guards-in-flow |
| [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) | Limit tab: form → `trade-limit-inline-guards` → My limits; CTA in `trade-ticket-submit-footer` |
| [`TradeTicketSubmitFooter.tsx`](../frontend-dapp/src/components/trade/TradeTicketSubmitFooter.tsx) | Shared footer wrapper |
| [`index.css`](../frontend-dapp/src/index.css) | `.trade-order-ticket-scroll`, `.trade-ticket-submit-footer` |
| [`LimitOrderEscrowPlaceGuardMessage.tsx`](../frontend-dapp/src/components/trade/LimitOrderEscrowPlaceGuardMessage.tsx) | Inline blocking / warning copy for place gates |
| [`trade-page-responsive.spec.ts`](../frontend-dapp/e2e/trade-page-responsive.spec.ts) | Playwright: footer opacity hit-test, guards outside footer, expiry clears footer |
| [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx) | RTL: guards precede footer in DOM (#500 / #527) |

## Rules of thumb

1. **Keep the money CTA visible** — docking to the ticket card bottom satisfies #348. Do not put the CTA back inside `.trade-order-ticket-scroll` with `position: sticky`.
2. **Opaque footer chrome** — `.trade-ticket-submit-footer` must use a solid underlay (`var(--bg-1)`) under `var(--panel-bg-strong)` (plus backdrop blur). **Never** use missing tokens like `--card` or a mostly transparent `color-mix` that lets ADVANCED / fee rows bleed through (same class of bug as sticky header opacity [#482](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/482)).
3. **Guards in normal document flow** — `LimitOrderEscrowPlaceGuardMessage` (`trade-limit-place-guard`, update-price guard) lives in `trade-limit-inline-guards` **above** the footer, not inside it. Blocking banners must not cover expiry / date inputs.
4. **Footer contents** — CTA button, broadcast pending link, and tx result alerts may stay in the footer. Do not re-home validation banners there without updating docs + E2E.
5. **Theme tokens** — use `--bg-1` / `--panel-bg-strong` (overridden in light theme). Do not introduce opaque panels that break light contrast.

## Related

- Sticky header opacity / shell clearance: [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md) (#482)
- Chrome mid-form float / ticket footer dock: [`AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md`](./AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md) ([#527](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/527))
- Trade workspace layout: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Limit price / place gates: [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md)
- Money CTA sizing / onboarding IA: [`AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md`](./AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md)
