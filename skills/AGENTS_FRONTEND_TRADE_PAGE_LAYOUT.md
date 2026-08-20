# Agent playbook: Trade page responsive layout

Use when changing **`TradePage.tsx`** sub-desktop grid, Tailwind breakpoints for **`md:`** / **`lg:`** on `/trade`, or Playwright tests in **`frontend-dapp/e2e/trade-page-responsive.spec.ts`**.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade page — responsive layout (sub-desktop)](../docs/frontend.md#trade-page-responsive-layout) | Breakpoint invariants for phone vs tablet vs desktop trade workspace ([GitLab **#146**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/146)) |
| [docs/frontend.md § Trade page — market context (tape, hybrid tag, limit-only book)](../docs/frontend.md#trade-page-market-context) | Last price headline, tape column labels, hybrid tooltip, order book scope, **Limit/Market tabs + post-only book-head preflight** ([GitLab **#149**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/149), [**#152**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/152)) |
| [docs/frontend.md § Trade route — onboarding IA](../docs/frontend.md#trade-route-onboarding-ia) | First-visit strip, money CTA sizing, collapsed tape/history ([GitLab **#417**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/417)) |
| [docs/frontend.md § Trade page — ticket footer CTA](../docs/frontend.md#trade-page-ticket-footer-cta) | Docked **Place limit** / Market footer (**T527-1–T527-10**), `#500` opacity + guards in flow ([GitLab **#527**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/527), [**#500**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/500)); agents: [`AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md`](./AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md), [`AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md`](./AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md) |
| [docs/frontend.md § Market data loading & outage (global)](../docs/frontend.md#market-data-loading-outage) | Shared banner/loading primitives across routes ([GitLab **#215**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/215)); agent: [`AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md`](./AGENTS_FRONTEND_MARKET_DATA_OUTAGE.md) |
| [docs/frontend.md § Trade page — indexer outage banner](../docs/frontend.md#trade-page-indexer-outage-banner) | Warning when market data service is down: **must align** with book/tape/chart reality; **no** `VITE_INDEXER_URL` in retail copy ([GitLab **#164**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/164), [**#174**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/174)) |
| [docs/frontend.md § Trade page — invalid pair deep link](../docs/frontend.md#trade-page-invalid-pair-link) | Non-`terra1` or malformed `/trade/:pairAddr` → URL cleanup, alert + pair-selector CTA ([GitLab **#176**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/176)) |
| [docs/frontend.md § Trade page — pair switch latency](../docs/frontend.md#trade-page-pair-switch-latency) | Parallel chart/book fetch, loading status, prefetch on selector intent ([GitLab **#180**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/180)) |
| [docs/frontend.md § Portal listboxes — layout stability](../docs/frontend.md#portal-listbox-layout-stability) | `#trade-pair-select` opens a fixed portaled menu without shifting chart/book/ticket ([GitLab **#181**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/181)) |
| [docs/frontend.md § Trade page — initial load / LCP](../docs/frontend.md#trade-page-initial-load) | Hard-reload skeleton, HTML bootstrap, legal footer deferral, Lighthouse LCP ([GitLab **#179**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/179)) |
| [docs/frontend.md § Trade page — limit place success affordances](../docs/frontend.md#trade-page-limit-place-success-affordances) | After successful **Place limit** on `/trade`: **View order** + **Place another** CTAs, scroll/highlight wiring to **My limits** ([GitLab **#161**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/161)) |
| [docs/frontend.md § Trade page — order book row actions](../docs/frontend.md#trade-book-row-actions) | Bids/asks **#id** column, per-row **Edit** / cancel, **Cancel all mine**, shared `useLimitOrderCancelMutation`; **one** `TradeOrderTicket` mount via `TRADE_DESKTOP_LAYOUT_MEDIA_QUERY` ([GitLab **#178**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/178), **#162**) |
| [docs/frontend.md § Trade page — limit order price field](../docs/frontend.md#trade-page-limit-order-price) | Tape reference, % deviation chips, headline-scaled USD, bid/ask submit gate ([GitLab **#154**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/154), [#488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/488)/[#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489)) |
| [docs/frontend.md § Limit place — Bid / Ask side control](../docs/frontend.md#limit-place-bid-ask-side) | Order ticket **Buy/Sell** control: radiogroup buttons, semantic fills (**#563**), test ids, keyboard model ([GitLab **#153**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/153)); agents: [`AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md`](./AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md), [`AGENTS_FRONTEND_TRADE_TICKET_HEADING.md`](./AGENTS_FRONTEND_TRADE_TICKET_HEADING.md) |
| [docs/frontend.md § Trade page — ticket heading](../docs/frontend.md#trade-page-ticket-heading) | Full **Buy {base}** heading, no compact wallet chip, green Buy / red Sell (**T563-1–T563-8**, [#563](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/563)) |
| [docs/frontend.md § Responsive shell & header navigation](../docs/frontend.md#responsive-header-navigation) | Same viewport bands for **header** density (`Layout.tsx`, `HEADER_FULL_NAV_MIN_WIDTH_PX` — [GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)) |
| `frontend-dapp/src/pages/TradePage.tsx` | `lg:hidden` grid vs `hidden lg:flex` CSS-grid desktop workspace ([#561](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/561)) |
| [`TradePageWorkspaceSkeleton.tsx`](../frontend-dapp/src/components/trade/TradePageWorkspaceSkeleton.tsx) / [`trade-bootstrap.css`](../frontend-dapp/public/bootstrap/trade-bootstrap.css) | Loading chrome must match the live desktop grid (tape as bottom row; no 24/52/24 nested chart/tape) |

## Rules of thumb

1. **Desktop (`lg:`+) is a CSS grid**, not `react-resizable-panels`. Product decision: [#561](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/561). Do **not** add `PanelResizeHandle` chrome to the default `/trade` layout. Hide book/ticket with toggles so the **chart** takes the vacated width. Tablet/phone behavior lives in the **`lg:hidden`** grid only.
2. **Prefer CSS grid placement** (`md:col-start-*`, `md:row-start-*`) over duplicating chart/ticket markup — one source of truth for queries and loading states.
3. If you add a **fourth** layout tier or JS-driven breakpoints, update **`docs/frontend.md`** invariants and extend **`trade-page-responsive.spec.ts`** so iPad portrait (`~820×1180`) and a narrow phone width stay covered.
4. **Playwright** must scope sub-desktop layout assertions under **`[data-testid="trade-sub-lg-workspace"]`** — the desktop workspace also contains an order book + chart and would otherwise duplicate roles.
5. **`PriceChart` embeds:** Wrappers around `PriceChart` on **`TradePage`** must stay **`flex flex-col min-h-0`** (and **`h-full`** in the desktop chart cell) so the candle pane is not clipped by **`overflow-hidden`** ([#151](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)). Do **not** wrap `PriceChart` (`shell-panel-strong`) in `card-glass` (**L561-1**).
6. **Panel prefs** persist as `'1'` / `'0'` only in [`tradeWorkspacePanels.ts`](../frontend-dapp/src/utils/tradeWorkspacePanels.ts). Corrupt strings fall back to defaults (book+ticket visible; tape collapsed). No query-param layout.
7. **One `TradeOrderTicket` mount** ([#178](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/178)). Hide with CSS + `inert` + `interactive={false}`; do not remount. Book **Edit** must re-show the ticket then apply the draft (**L561-11**).

## Invariants (GitLab #561)

| Id | Rule |
|----|------|
| **L561-1** | Desktop chart is a single `PriceChart` `shell-panel-strong` surface — no wrapping `card-glass`. |
| **L561-2** | Design-system + this skill: one chrome layer per region. Swap / Pool / Limits / Charts audited. |
| **L561-3** | Desktop Recent trades is an independent bottom-row panel, not inside the chart stack. No chart↔tape splitter. |
| **L561-4** | No `PanelResizeHandle` (or equivalent drag chrome) in the default `/trade` layout. P10 asserts absence. |
| **L561-5** | Hide Order book → chart expands (`3.2fr`); remaining ticket stays `1fr`. Restore control stays visible. |
| **L561-6** | Hide Order ticket → chart expands; remaining book stays `1fr`. Hidden ticket is inert (no focus, no submit). Restore control stays visible. |
| **L561-7** | Hiding both side panels gives the chart the full workspace width. Default (both visible: `1fr` / `2.2fr` / `1fr`) keeps the chart the largest region. |
| **L561-8** | Tape expand/collapse persists. Book/ticket visibility persists. Corrupt `localStorage` → defaults. |
| **L561-9** | Sub-`lg` grid structure unchanged ([#146](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/146)); chart double-wrap flattened. Exactly one ticket mount. |
| **L561-10** | Visible ticket keeps footer CTA dock ([#527](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/527)). Candles not clipped ([#151](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)). Pair selector portal does not shift layout ([#181](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/181)). |
| **L561-11** | Book **Edit** while ticket hidden re-shows the ticket and applies the draft. Pair URL change remounts via Layout keyed Outlet so a hidden ticket still binds the current `pairAddr` (A9). |
| **L561-12** | Docs/skills match this layout. `make verify-issue-561` covers unit + targeted e2e. |

**Hide vs unmount:** desktop book/ticket stay mounted (`hidden` + `inert`). Form fields persist across hide/show. Pair switch still updates `pairAddr` on the mounted ticket (**A9**).

**Chrome audit (#561 L2):** Swap nests `card-glass` IO cards inside a page `shell-panel-strong` (allowed distinct blocks). Pool uses `shell-panel*` without wrapping `PriceChart`. Charts mounts `PriceChart` without extra `card-glass`. Limits extra `card-glass` around `OrderBookPanel` was removed.

Verify: `make verify-issue-561`.

## Related

- Anti-cognitive-overload retail copy: [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) ([#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/489))
- Hard reload / LCP / workspace skeleton: [GitLab **#179**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/179), [`AGENTS_FRONTEND_TRADE_INITIAL_LOAD.md`](./AGENTS_FRONTEND_TRADE_INITIAL_LOAD.md) (desktop skeleton + HTML bootstrap follow this layout).
- **Chart `getPair` Retry after indexer 404:** [GitLab **#177**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/177), [docs/frontend.md § Trade page — chart pair fetch retry](../docs/frontend.md#trade-page-chart-retry), [`AGENTS_FRONTEND_QUERY_RETRY.md`](./AGENTS_FRONTEND_QUERY_RETRY.md).

- Limit **Bid / Ask** side control (order ticket): [`AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md`](./AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md)
- Ticket heading + Buy/Sell colors (no compact wallet chip): [`AGENTS_FRONTEND_TRADE_TICKET_HEADING.md`](./AGENTS_FRONTEND_TRADE_TICKET_HEADING.md) ([#563](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/563))
- Post-merge Coolify cut: [`AGENTS_POST_MERGE_STACK.md`](./AGENTS_POST_MERGE_STACK.md) ([#573](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/573))
- User-facing error strings (`getErrorMessage`, `RetryError`): [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md); trade outage banner copy: [`../frontend-dapp/src/utils/indexerTradeOutageCopy.ts`](../frontend-dapp/src/utils/indexerTradeOutageCopy.ts) ([GitLab **#164**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/164), [**#174**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/174))
- Responsive header / nav: [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md)
- Price chart (lightweight-charts overlays, USD scale, volume): [`AGENTS_FRONTEND_PRICE_CHART.md`](./AGENTS_FRONTEND_PRICE_CHART.md); [`docs/frontend.md`](../docs/frontend.md) § *Trade page — price chart invariants* ([GitLab **#113**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/113), [**#150**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/150), [**#151**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151))
- Pair display invert (one state for chart + ticket, convert-on-submit): [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) ([GitLab **#524**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/524)); [`docs/frontend.md`](../docs/frontend.md) § *Trade pair display invert*
- Limit price field (tape reference, validation, USD anchor): [`AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md`](./AGENTS_FRONTEND_LIMIT_ORDER_PRICE.md) ([GitLab **#154**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/154))
- Order book row cancel / edit / cancel-all: [`AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md`](./AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md) ([GitLab **#162**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/162))
- Limit place success CTAs (**View order**, **Place another**) on `/trade`: [`docs/frontend.md`](../docs/frontend.md) § *Trade page — limit place success affordances* ([GitLab **#161**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/161)); code in [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx)
- Ticket money-CTA dock (Chrome float) + footer opacity / guards: [`AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md`](./AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md) ([#527](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/527)), [`AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md`](./AGENTS_FRONTEND_TRADE_LIMIT_STICKY_CTA.md) ([#500](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/500))
- Wallet swap / limit history + CSV on `/trade` and `/limits`: [`AGENTS_FRONTEND_ORDER_HISTORY.md`](./AGENTS_FRONTEND_ORDER_HISTORY.md) ([GitLab **#163**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/163))
- Trade onboarding IA / CTA hierarchy / progressive disclosure: [`AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md`](./AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md) ([GitLab **#417**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/417))
- Invalid `/trade/:pairAddr` deep links (notice + CTA, no garbage in URL): [`AGENTS_FRONTEND_TRADE_INVALID_PAIR_LINK.md`](./AGENTS_FRONTEND_TRADE_INVALID_PAIR_LINK.md) ([GitLab **#176**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/176))
- Pair switch latency (parallel fetch, loading banner, prefetch): [`AGENTS_FRONTEND_TRADE_PAIR_SWITCH.md`](./AGENTS_FRONTEND_TRADE_PAIR_SWITCH.md) ([GitLab **#180**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/180))
- Pair selector CLS / portaled `MenuSelect`: [`AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md`](./AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md) ([GitLab **#181**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/181))
