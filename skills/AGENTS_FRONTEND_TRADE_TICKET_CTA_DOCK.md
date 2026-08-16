# Agent playbook: Trade ticket money-CTA dock (Chrome float)

Use when changing **`/trade` order-ticket** **Place limit** / **Update price** / **Market buy|sell** chrome so the money CTA docks to the **bottom of the ticket column** instead of floating mid-form on Chromium ([GitLab **#527**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/527)).

`#500` opacity / guards-in-flow / `#348` visibility still apply. Do **not** treat `position: sticky; bottom: 0` inside `.trade-order-ticket-scroll` as the end state.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#527**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/527) | Full spec: current code, guardrails, AC, path tests, attack plan |
| [docs/frontend.md § Trade page — ticket footer CTA](../docs/frontend.md#trade-page-ticket-footer-cta) | Live **T527-1–T527-10** invariants |
| [`AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md`](./AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md) | `#500` opacity, guards-in-flow (footer payload) |
| [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) | Ticket card: scroll body + `TradeTicketSubmitFooter` sibling |
| [`TradeTicketSubmitFooter.tsx`](../frontend-dapp/src/components/trade/TradeTicketSubmitFooter.tsx) | Shared footer wrapper + Market submit chrome |
| [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) | `dockSubmit` publishes Market CTA to the footer |
| [`index.css`](../frontend-dapp/src/index.css) | `.trade-order-ticket-scroll`, `.trade-ticket-submit-footer` |
| [`trade-page-responsive.spec.ts`](../frontend-dapp/e2e/trade-page-responsive.spec.ts) | Playwright `#348` / `#500` / `#527` geometry |

## Target layout

```text
.card-glass.flex.flex-col.h-full.min-h-0          /* data-testid=trade-order-ticket-card */
  header (shrink-0)
  .trade-order-ticket-scroll (flex-1 min-h-0 overflow-y-auto)
    tabs, side, fields, expiry, advanced, summary
    .trade-limit-inline-guards
    My open limits
  .trade-ticket-submit-footer (shrink-0, opaque, border-top)
    Place limit | Update price | Market buy/sell | Connect Wallet
```

Prefer `flex` + `shrink-0`. Do **not** use `position: fixed` or a portal footer.

## Invariants (T527-1–T527-10)

1. **T527-1 Dock, do not float** — at ticket scroll-top on Chromium, the CTA bottom edge aligns with the ticket card bottom (≤ 8px). It must not intersect Pay % chips, Receive, or Expiry.
2. **T527-2 Shared footer** — Limit + Market share `trade-ticket-submit-footer`. Do not leave Market as a second in-flow button.
3. **T527-3 Keep `#348` visibility** — desktop ~1280×720 and tablet chart\|ticket row: money CTA visible without scrolling the ticket body.
4. **T527-4 Keep `#500`** — opaque `var(--bg-1)` under `var(--panel-bg-strong)`; guards stay in normal flow **above** the footer; footer holds CTA + pending link + tx alerts only.
5. **T527-5 No sticky / fixed / portal** — footer is a sibling of the scrollport inside the card.
6. **T527-6 Layout only** — no change to `place_limit_order`, invert convert-on-submit (`#524`), crossing / pause / blacklist / gas gates.
7. **T527-7 One ticket mount** — do not remount a second `TradeOrderTicket` (`#178`).
8. **T527-8 z-index** — footer stays under wallet modal, risk/NFA, clickwrap, toasts, `#trade-pair-select` (`#181`).
9. **T527-9 Testids** — keep `trade-limit-submit`, `trade-limit-update-price-submit`, `trade-limit-inline-guards`, `trade-order-ticket-scroll`, `trade-market-submit`. Footer is `trade-ticket-submit-footer`.
10. **T527-10 `/limits` standalone** is out of scope unless a helper is shared.

## Rules of thumb

1. **Dock, do not float** — see T527-1.
2. **Market uses `dockSubmit`** — `TradeMarketOrderPanel` still owns quote/submit state; standalone tests render the CTA in-flow when `dockSubmit` is false.
3. **Do not resurrect `.trade-limit-submit-sticky`** or `--trade-limit-sticky-clearance`. Extra scroll end-padding is unnecessary with a sibling footer.
4. **Phone** may keep the CTA in normal flow under the form (no mid-form overlay, no `position: fixed` over the bottom nav).

## Verify

```bash
make verify-issue-527
make test-frontend
# Playwright Chromium (needs LocalTerra + .env.local):
# e2e/trade-page-responsive.spec.ts — assert submit.bottom ≈ ticketCard.bottom
```

Issue: [GitLab **#527**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/527) (AC1–AC8, Playwright P1–P11, attack A1–A10).

## Related

- Sticky opacity / guards: [`AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md`](./AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md) (`#500`)
- Trade workspace layout: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Money CTA sizing: [`AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md`](./AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md) (`#417`)
- Pair invert convert-on-submit: [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) (`#524`)
- Slippage chips stay in the ticket body (not this footer): [`AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md`](./AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md) (`#528`)
