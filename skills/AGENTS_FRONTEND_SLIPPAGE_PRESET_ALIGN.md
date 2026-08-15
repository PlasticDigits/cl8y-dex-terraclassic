# Agent playbook: Slippage protection preset alignment (GitLab #528)

Use when changing **Slippage protection** preset chips on `/trade` Market or Swap Settings so **0.5% / 1% / 5%** stay one aligned group instead of orphaning the first chip next to the label ([GitLab **#528**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/528)).

Issue **#528 is open**. `#497` default 5% + shared preset values still apply. Do **not** treat a `flex flex-wrap` list that mixes the label (or Custom input) with the three chips as done.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#528**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/528) | Full spec: current wrap, guardrails, AC, path tests, attack plan |
| [`AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md`](./AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md) | Default 5%, `[0.5, 1.0, 5.0]`, `max_spread` mapping |
| [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) | Reported wrap: label + chips in one `flex-wrap` |
| [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) | Settings: label on its own line; chips + Custom still wrap together |
| [`tradeMoneyCta.ts`](../frontend-dapp/src/utils/tradeMoneyCta.ts) | `TRADE_SLIPPAGE_PRESET_CLASS` (`min-h-11`, `#417`) |
| [`slippageProtectionCopy.ts`](../frontend-dapp/src/utils/slippageProtectionCopy.ts) | Label, default, presets |
| [`trade-page-responsive.spec.ts`](../frontend-dapp/e2e/trade-page-responsive.spec.ts) | Viewport geometry — extend for chip baseline |

## Current defect

On a narrow `/trade` ticket, `flex flex-wrap` fits **Slippage protection:** + **0.5%** on the first line and wraps **1%** / **5%**. The first chip sits on the label row. Swap Settings can orphan the same first chip when the Custom field shares the wrap list.

## Target layout

Label is **not** a wrap sibling of the chips. Three presets are one `role="group"` with a shared baseline:

```text
Slippage protection
[ 0.5% ] [ 1% ] [ 5% ]     ← same top/bottom (≤ 2px); Trade: min-h-11
                           Swap Custom sits outside this group (may wrap below)
```

Prefer label above + `flex items-center gap-2` or `grid grid-cols-3`. If the ticket is too narrow, shrink as a 3-up grid or wrap **as a group** — never leave 0.5% on the label row.

## Rules of thumb

1. **Layout only** — do not change `[0.5, 1.0, 5.0]`, default **5%**, clamp `0.01`–`50`, or `max_spread = percent / 100`. Do not hard-code `0.5` as the product default (`#497`).
2. **No Custom on Trade Market** unless product already asked. Swap keeps Custom + range error + high-warn only when **strictly greater than** 5%.
3. **Keep `#417` touch targets** — Trade chips stay `TRADE_SLIPPAGE_PRESET_CLASS`. Do not shrink to fit the label row.
4. **Label** stays `SLIPPAGE_PROTECTION_LABEL` (`#412`). No `*-neo` (`#488`). No `token0` / `max_spread` in retail copy (`#489`).
5. **Testids** — keep `trade-market-slippage-preset-0.5`, `…-1`, `…-5`. Add a group testid (e.g. `trade-market-slippage-presets`) for geometry.
6. **Chips stay in the ticket body** — not in the `#527` money-CTA footer. `elementFromPoint` on a chip must hit that chip.
7. **Pool withdraw** `0.5 / 1.0 / 2.0` is out of scope unless a shared helper is reused without changing those values.
8. **Update this file’s “open” note** when `#528` closes.

## Verify

Issue: [GitLab **#528**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/528) (AC1–AC9, Playwright P1–P10, attack A1–A10). After implement: `make verify-issue-528` (add the script in the same MR).

```bash
make test-frontend
# Playwright Chromium: chip bounding boxes share y (±2px); 0.5% is not on the label row
```

## Related

- Default + mapping: [`AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md`](./AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md) (`#497`)
- Ticket footer must not cover chips: [`AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md`](./AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md) (`#527`)
- Design tokens / `tab-glass*`: [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md) (`#488`)
- Focus rings: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md) (`#144`)
