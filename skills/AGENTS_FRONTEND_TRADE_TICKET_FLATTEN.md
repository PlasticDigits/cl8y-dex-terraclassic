# Agent playbook: `/trade` ticket Market default + flatten + token wash

Use when changing the **`/trade` order ticket** default tab, Market/Limit switcher chrome, heading logo/wash, Side/Top-buy nesting, Market/Limit `TicketSection` cards, or where **Slippage protection** / limit expiry live ([GitLab **#693**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/693)).

Styling / IA only. **Do not** change escrow mapping, quote payloads, hybrid always-on, footer CTA dock, or on-chain `side`.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#693**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/693) | Full spec: Market default, compact tabs, logo wash, flatten, Advanced |
| [docs/frontend.md § Trade page — ticket flatten](../docs/frontend.md#trade-page-ticket-flatten) | **T693-1–T693-8** |
| [docs/frontend.md § Trade page — ticket heading](../docs/frontend.md#trade-page-ticket-heading) | **T563-6** amended: tabs are compact text, not `tab-glass*` |
| [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) | Default `orderTab`, text tabs, logo, wash, unwrapped Side |
| [`TradeMarketOrderPanel.tsx`](../frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx) | Pay + expected receive on default path; slippage/quote extras in Advanced |
| [`tokenHeadingWash.ts`](../frontend-dapp/src/utils/tokenHeadingWash.ts) / [`useTokenHeadingWash.ts`](../frontend-dapp/src/hooks/useTokenHeadingWash.ts) | Clamped wash; never leftover orange |
| [`TokenLogo.tsx`](../frontend-dapp/src/components/ui/TokenLogo.tsx) | Allowlisted `<img>` + blockie (#378) |
| [`AGENTS_FRONTEND_TRADE_TICKET_HEADING.md`](./AGENTS_FRONTEND_TRADE_TICKET_HEADING.md) | Heading wrap + green/red Buy/Sell (**T563**) |
| [`AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md`](./AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md) | 0.5/1/5 chips under Advanced when open (#528) |

## Invariants (T693-1–T693-8)

1. **T693-1 Default Market** — Fresh `/trade/:pair` (`terra1…`) selects Market (`trade-order-tab-market` `aria-selected="true"`). Footer is Market CTA. No `limit-order-price-input` until Limit. Book **Edit** / limit draft still `setOrderTab('limit')`. **Place another** stays on Limit. Pair remount → Market again.
2. **T693-2 Compact tabs** — Market/Limit are `.trade-order-text-tab` (intrinsic width, selected underline + `aria-selected`). Not `tab-glass*` pills. Not a full-width 2-col bordered toolbar. `:focus-visible` ring required (#144).
3. **T693-3 Heading logo + wash** — Selected pair: sibling `TokenLogo` (allowlist / blockie) for the **displayed** base + `{verb} {displayBase}` text nodes (no `dangerouslySetInnerHTML`). Wash from displayed-base id (hash) or local canvas sample of an allowlisted https logo. **Never** `rgba(251, 146, 60, …)`. `Select a pair` = no logo, neutral wash. Invert updates logo + verb together. Buy/Sell and money CTAs are **not** painted with the wash.
4. **T693-4 Flatten** — No `TicketSection` / `TicketStat` on this ticket. No **Side** title. No **Top buy** / **Top sell**. Buy/Sell radiogroup is a direct child of the ticket scroll. One **Market** + Docs or **Limit** + Docs line (`trade-order-mode-docs`). No inner `<h3>Market</h3>`.
5. **T693-5 Advanced Market** — Default path: Pay + Max + one expected-receive line + blocking errors (pause, freeze, blacklist, tax extra-debit, quote error, book-exceeds-pay). Slippage chips (`trade-market-slippage-*`), min-after-slippage, Route extras, pre-submit card live under **Advanced** (open). Closed Advanced still applies store slippage (default **5%**) to `max_spread`.
6. **T693-6 Advanced Limit** — Default path: price + Pay + Receive + invalid buy/sell alert. Expiry, % chips + ref/USD, pre-submit fee card under Advanced. `/limits` place card stays Limit-only with chips next to the price input.
7. **T693-7 Behavior unchanged** — Bid escrows token1; ask token0. Click Buy still `onSideChange('bid')`. Quotes GET `/route/solve` (typed book → POST). Hybrid always on (#596). Footer dock (#527) stays a sibling footer — leftover Playwright clips T527-1 overlap to **visible** `trade-order-ticket-scroll` when Advanced is open ([#702](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/702)). One ticket mount (#178). Hidden ticket stays `inert`.
8. **T693-8 Docs** — this playbook + `docs/frontend.md` + `docs/design-system.md` + QA **10.2.19**. Verify: `make verify-issue-693`.

## Rules of thumb

1. **Do not auto-submit** on Market default. Footer stays disabled / Connect Wallet until amount + gates (A1).
2. **Logo color is a muted wash, not a trust signal.** Clamp luminance. Canvas extract only from allowlisted `https` images; tainted → hash fallback; no third-party color API (A3/A10).
3. **Do not restyle `/limits`** except via a shared primitive with a `/trade`-only prop (`showDeviationChrome={false}`).
4. **Do not add** “Market is now the default” lecture banners.
5. **T563-6 is amended here** — Limit/Market tabs are compact text, not `tab-glass*`. Place/Market CTAs stay `btn-primary`. Buy/Sell stay `side-control*`.

## Verify

```bash
make verify-issue-693
make verify-issue-563
make verify-issue-528
python3 scripts/check_design_tokens.py
python3 scripts/check_chrome_nesting.py
```

Issue: [GitLab **#693**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/693) (AC **T1–T11**, attack **A1–A11**). Coolify leftover: [#702](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/702) — [`AGENTS_POST_MERGE_OPS_702.md`](./AGENTS_POST_MERGE_OPS_702.md). Do **not** reopen #693 for ops/QA.

## Related

- Heading + Buy/Sell colors: [`AGENTS_FRONTEND_TRADE_TICKET_HEADING.md`](./AGENTS_FRONTEND_TRADE_TICKET_HEADING.md) (#563)
- Footer dock: [`AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md`](./AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md) (#527)
- Desktop layout / one mount: [`AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md`](./AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md) (#561)
- One chrome layer: [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) (#653)
- Slippage chips: [`AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md`](./AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md) (#528)
- Hybrid always on: [`AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md`](./AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md) (#596)
- Pair invert: [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) (#524)
- Logo allowlist: [`TokenLogo.tsx`](../frontend-dapp/src/components/ui/TokenLogo.tsx) (#378)
