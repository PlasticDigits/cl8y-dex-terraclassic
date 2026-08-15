# Agent playbook: Trade ticket money-CTA dock (Chrome float)

Use when changing **`/trade` order-ticket** **Place limit** / **Update price** / **Market buy|sell** chrome so the money CTA docks to the **bottom of the ticket column** instead of floating mid-form on Chromium ([GitLab **#527**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/527)).

Issue **#527 is open**. `#500` opacity / guards-in-flow / `#348` visibility still apply. Do **not** treat `position: sticky; bottom: 0` inside `.trade-order-ticket-scroll` as the end state.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#527**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/527) | Full spec: current code, guardrails, AC, path tests, attack plan |
| [docs/frontend.md § Trade page — limit ticket sticky CTA](../docs/frontend.md#trade-page-limit-ticket-sticky-cta) | Live `#500` invariants until the footer MR lands |
| [`AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md`](./AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md) | `#500` opacity, clearance, guards-in-flow |
| [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) | Limit tab: form → `trade-limit-inline-guards` → `trade-limit-submit-sticky` |
| [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) | Market CTA is in-flow today (not sticky) |
| [`index.css`](../frontend-dapp/src/index.css) | `.trade-order-ticket-scroll`, `.trade-limit-submit-sticky`, `--trade-limit-sticky-clearance` |
| [`trade-page-responsive.spec.ts`](../frontend-dapp/e2e/trade-page-responsive.spec.ts) | Playwright `#348` / `#500` geometry — extend for bottom alignment |

## Current defect

On Chrome, `.trade-limit-submit-sticky` (`position: sticky; bottom: 0`) inside a flex + `overflow-y-auto` ticket (desktop `react-resizable-panels`) paints **mid-column**. **Place limit** sits between Pay **25%/50%/75%/100%** chips and Expiry (**No expiry / 24h / 7d**) and overlays Receive. `#500` fixed bleed-through, not Chromium’s sticky containing block.

## Target layout

Replace sticky-inside-scroll with a **true ticket footer** (Limit + Market share one slot):

```text
.card-glass.flex.flex-col.h-full.min-h-0
  header (shrink-0)
  .trade-order-ticket-scroll (flex-1 min-h-0 overflow-y-auto)
    tabs, side, fields, expiry, advanced, summary
    .trade-limit-inline-guards
    My open limits
  .trade-ticket-submit-footer (shrink-0, opaque, border-top)
    Place limit | Update price | Market buy/sell | Connect Wallet
```

Prefer `flex` + `shrink-0`. Do **not** use `position: fixed` or a portal footer.

## Rules of thumb

1. **Dock, do not float** — at ticket scroll-top on Chromium, the CTA bottom edge aligns with the ticket card bottom (≤ 8px). It must not intersect Pay % chips, Receive, or Expiry.
2. **Keep `#348` visibility** — desktop ~1280×720 and tablet chart\|ticket row: money CTA visible without scrolling the ticket body.
3. **Keep `#500` invariants** — opaque `var(--bg-1)` under `var(--panel-bg-strong)`; guards stay in normal flow **above** the footer; footer holds CTA + pending link + tx alerts only.
4. **Layout only** — no change to `place_limit_order`, invert convert-on-submit (`#524`), crossing / pause / blacklist / gas gates.
5. **One ticket mount** — do not remount a second `TradeOrderTicket` (`#178`).
6. **z-index** — footer stays under wallet modal, risk/NFA, clickwrap, toasts, `#trade-pair-select` (`#181`).
7. **`/limits` standalone** is out of scope unless a helper is shared.
8. **Testids** — keep `trade-limit-submit`, `trade-limit-update-price-submit`, `trade-limit-inline-guards`, `trade-order-ticket-scroll`, `trade-market-submit`. Rename `trade-limit-submit-sticky` only if every test + this playbook + `docs/frontend.md` update in the same MR.
9. **Rewrite the `#500` section** in `docs/frontend.md` in the implementation MR: “sticky CTA” → “ticket footer CTA”. Update this file’s “open” note when `#527` closes.

## Verify

Issue: [GitLab **#527**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/527) (AC1–AC8, Playwright P1–P11, attack A1–A10). After implement: `make verify-issue-527` (add the script in the same MR).

```bash
make test-frontend
# Playwright Chromium: e2e/trade-page-responsive.spec.ts — assert submit.bottom ≈ ticketCard.bottom
```

## Related

- Sticky opacity / guards: [`AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md`](./AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md) (`#500`)
- Trade workspace layout: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md)
- Money CTA sizing: [`AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md`](./AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md) (`#417`)
- Pair invert convert-on-submit: [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) (`#524`)
