# Agent playbook: Slippage protection preset alignment (GitLab #528)

Use when changing **Slippage protection** preset chips on `/trade` Market or Swap Settings so **0.5% / 1% / 5%** stay one aligned group instead of orphaning the first chip next to the label ([GitLab **#528**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/528)).

Issue **#528 is implemented** (shared `SlippageProtectionPresets`). `#497` default 5% + shared preset values still apply. Do **not** treat a `flex flex-wrap` list that mixes the label (or Custom input) with the three chips as done.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#528**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/528) | Full spec: wrap defect, guardrails, AC, path tests, attack plan |
| [`docs/frontend.md` § Slippage protection preset alignment](../docs/frontend.md#slippage-protection-preset-align) | Invariants **S528-1–S528-10** |
| [`AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md`](./AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md) | Default 5%, `[0.5, 1.0, 5.0]`, `max_spread` mapping |
| [`SlippageProtectionPresets.tsx`](../frontend-dapp/src/components/common/SlippageProtectionPresets.tsx) | Shared label-above + `grid-cols-3` group |
| [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) | Trade Market: three chips, `TRADE_SLIPPAGE_PRESET_CLASS` |
| [`SwapPage.tsx`](../frontend-dapp/src/pages/SwapPage.tsx) | Settings: same group; Custom sibling **outside** the group |
| [`tradeMoneyCta.ts`](../frontend-dapp/src/utils/tradeMoneyCta.ts) | `TRADE_SLIPPAGE_PRESET_CLASS` (`min-h-11`, `#417`) |
| [`slippageProtectionCopy.ts`](../frontend-dapp/src/utils/slippageProtectionCopy.ts) | Label, default, presets, Custom sanitize |
| [`slippage-preset-align-528.spec.ts`](../frontend-dapp/e2e/slippage-preset-align-528.spec.ts) | Playwright P1–P10 geometry |

## Target layout

Label is **not** a wrap sibling of the chips. Three presets are one `role="group"` with a shared baseline:

```text
Slippage protection
[ 0.5% ] [ 1% ] [ 5% ]     ← same top/bottom (≤ 2px); Trade: min-h-11
[ Custom ]                 ← Swap only; stacks below the group, never between chips
```

Use `grid grid-cols-3` on the group so chips shrink together on a narrow ticket. Never leave 0.5% on the label row.

## Rules of thumb

1. **Layout only** — do not change `[0.5, 1.0, 5.0]`, default **5%**, clamp `0.01`–`50`, or `max_spread = percent / 100`. Do not hard-code `0.5` as the product default (`#497`).
2. **No Custom on Trade Market** unless product already asked. Swap keeps Custom + range error + high-warn only when **strictly greater than** 5%.
3. **Keep `#417` touch targets** — Trade chips stay `TRADE_SLIPPAGE_PRESET_CLASS`. Do not shrink to fit the label row.
4. **Label** stays `SLIPPAGE_PROTECTION_LABEL` (`#412`). No `*-neo` (`#488`). No `token0` / `max_spread` in retail copy (`#489`).
5. **Testids** — keep `trade-market-slippage-preset-0.5`, `…-1`, `…-5`. Group: `trade-market-slippage-presets` / `swap-slippage-presets`. Swap Custom: `swap-slippage-custom`.
6. **Chips stay in the ticket body** — not in the `#527` money-CTA footer. `elementFromPoint` on a chip must hit that chip.
7. **Pool withdraw** `0.5 / 1.0 / 2.0` is out of scope unless a shared helper is reused without changing those values.
8. **Custom sanitize** — `sanitizeSlippageCustomInput` (digits + one `.`). Do not persist `< 0.01`; clamp `> 50` to 50.

## Verify

```bash
make verify-issue-528
```

```bash
make test-frontend
# Playwright Chromium: chip bounding boxes share y (±2px); 0.5% is not on the label row
```

## Related

- Default + mapping: [`AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md`](./AGENTS_FRONTEND_DEFAULT_SLIPPAGE.md) (`#497`)
- Ticket footer must not cover chips: [`AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md`](./AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md) (`#527`)
- Design tokens / `tab-glass*`: [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md) (`#488`)
- Focus rings: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md) (`#144`)
- Touch targets: [`AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md`](./AGENTS_FRONTEND_TRADE_ONBOARDING_IA.md) (`#417`)
