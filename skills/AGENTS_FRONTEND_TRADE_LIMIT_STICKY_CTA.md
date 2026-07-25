# Agent playbook: Trade limit ticket sticky CTA

Use when changing the **`/trade` order ticket** sticky **Place limit** / **Update price** chrome, ticket scroll clearance, or inline place/price/gas guard placement relative to that footer ([GitLab **#500**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/500)).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade page — limit ticket sticky CTA](../docs/frontend.md#trade-page-limit-ticket-sticky-cta) | Invariants for opaque sticky CTA, scroll clearance, guards in flow |
| [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) | Limit tab: form → `trade-limit-inline-guards` → `trade-limit-submit-sticky` → My limits |
| [`index.css`](../frontend-dapp/src/index.css) | `.trade-order-ticket-scroll`, `.trade-limit-submit-sticky`, `--trade-limit-sticky-clearance` |
| [`LimitOrderEscrowPlaceGuardMessage.tsx`](../frontend-dapp/src/components/trade/LimitOrderEscrowPlaceGuardMessage.tsx) | Inline blocking / warning copy for place gates |
| [`trade-page-responsive.spec.ts`](../frontend-dapp/e2e/trade-page-responsive.spec.ts) | Playwright: sticky opacity hit-test, guards outside sticky, expiry clears footer |
| [`TradePage.test.tsx`](../frontend-dapp/src/pages/TradePage.test.tsx) | RTL: guards precede sticky in DOM (#500) |

## Rules of thumb

1. **Keep the sticky CTA** — pinning **Place limit** is intentional (#348 visibility). When the ticket is fully scrolled, the footer should settle in-flow above **My open limits**.
2. **Opaque sticky chrome** — `.trade-limit-submit-sticky` must use a solid underlay (`var(--bg-1)`) under `var(--panel-bg-strong)` (plus backdrop blur). **Never** use missing tokens like `--card` or a mostly transparent `color-mix` that lets ADVANCED / fee rows bleed through (same class of bug as sticky header opacity [#482](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/482)).
3. **Guards in normal document flow** — `LimitOrderEscrowPlaceGuardMessage` (`trade-limit-place-guard`, update-price guard) lives in `trade-limit-inline-guards` **above** the sticky footer, not inside it. Blocking banners must not cover expiry / date inputs.
4. **Scroll clearance** — ticket body uses `.trade-order-ticket-scroll` with `--trade-limit-sticky-clearance` end padding / `scroll-padding-bottom` so expiry, datetime, and ADVANCED can scroll clear of the pinned CTA. Prefer CSS on that class over Tailwind `p-4` so clearance wins.
5. **Sticky contents** — CTA button, broadcast pending link, and tx result alerts may stay in the sticky chrome. Do not re-home validation banners there without updating docs + E2E.
6. **Theme tokens** — use `--bg-1` / `--panel-bg-strong` (overridden in light theme). Do not introduce opaque panels that break light contrast.

## Related

- Sticky header opacity / shell clearance: [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md) (#482)
- Trade workspace layout: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Limit price / place gates: [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md)
- Money CTA sizing / onboarding IA: [`AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md`](./AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md)
