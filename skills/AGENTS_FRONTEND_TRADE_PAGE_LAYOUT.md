# Agent playbook: Trade page responsive layout

Use when changing **`TradePage.tsx`** sub-desktop grid, Tailwind breakpoints for **`md:`** / **`lg:`** on `/trade`, or Playwright tests in **`frontend-dapp/e2e/trade-page-responsive.spec.ts`**.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Trade page — responsive layout (sub-desktop)](../docs/frontend.md#trade-page-responsive-layout) | Breakpoint invariants for phone vs tablet vs desktop trade workspace ([GitLab **#146**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/146)) |
| [docs/frontend.md § Trade page — market context (tape, hybrid tag, limit-only book)](../docs/frontend.md#trade-page-market-context) | Last price headline, tape column labels, hybrid tooltip, order book scope ([GitLab **#149**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/149)) |
| [docs/frontend.md § Responsive shell & header navigation](../docs/frontend.md#responsive-header-navigation) | Same viewport bands for **header** density (`Layout.tsx`, `HEADER_FULL_NAV_MIN_WIDTH_PX` — [GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)) |
| `frontend-dapp/src/pages/TradePage.tsx` | `lg:hidden` grid vs `hidden lg:block` resizable `PanelGroup` |

## Rules of thumb

1. **Keep desktop (`lg:`+) on `react-resizable-panels`** unless there is a product decision to replace it; tablet/phone behavior lives in the **`lg:hidden`** grid only.
2. **Prefer CSS grid placement** (`md:col-start-*`, `md:row-start-*`) over duplicating chart/ticket markup — one source of truth for queries and loading states.
3. If you add a **fourth** layout tier or JS-driven breakpoints, update **`docs/frontend.md`** invariants and extend **`trade-page-responsive.spec.ts`** so iPad portrait (`~820×1180`) and a narrow phone width stay covered.
4. **Playwright** must scope layout assertions under **`[data-testid="trade-sub-lg-workspace"]`** — the `hidden lg:block` desktop workspace duplicates chart/order-book headings in the DOM.
5. **`PriceChart` embeds:** Chart card wrappers on **`TradePage`** must stay **`flex flex-col min-h-0`** (and **`h-full`** in the desktop chart `Panel`) so the candle pane is not clipped by **`overflow-hidden`** when the panel is shorter than header + old fixed chart height ([#151](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151)).

## Related

- User-facing error strings (`getErrorMessage`, `RetryError`): [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md)
- Responsive header / nav: [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md)
- Price chart (lightweight-charts overlays, USD scale, volume): [`AGENTS_FRONTEND_PRICE_CHART.md`](./AGENTS_FRONTEND_PRICE_CHART.md); [`docs/frontend.md`](../docs/frontend.md) § *Trade page — price chart invariants* ([GitLab **#113**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/113), [**#150**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/150), [**#151**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/151))
