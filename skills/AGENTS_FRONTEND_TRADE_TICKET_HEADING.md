# Agent playbook: Trade ticket heading + Buy/Sell side colors

Use when changing **`/trade` order-ticket heading**, the compact ticket-header wallet chip, or **Buy/Sell** side-control colors on **`/trade`** or **`/limits`** ([GitLab **#563**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/563)).

Styling only. **Do not** change escrow mapping, quote payloads, or on-chain `side`.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [GitLab **#563**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/563) | Full spec: truncation, redundant connect chip, green Buy / red Sell |
| [docs/frontend.md § Trade page — ticket heading](../docs/frontend.md#trade-page-ticket-heading) | **T563-1–T563-8** heading + chip invariants |
| [docs/frontend.md § Limit place — Bid / Ask side](../docs/frontend.md#limit-place-bid-ask-side) | Radiogroup + semantic fill exception |
| [docs/design-system.md](../docs/design-system.md) | Side-control exception to “semantic tokens are not button fills” |
| [`TradeOrderTicket.tsx`](../frontend-dapp/src/components/trade/TradeOrderTicket.tsx) | Heading + header; footer Connect Wallet stays |
| [`LimitOrderBidAskSideSelector.tsx`](../frontend-dapp/src/components/trade/LimitOrderBidAskSideSelector.tsx) | Shared Buy/Sell control (`side-buy*` / `side-sell*`) |
| [`TradeTicketSubmitFooter.tsx`](../frontend-dapp/src/components/trade/TradeTicketSubmitFooter.tsx) | Disconnected money-path **Connect Wallet** (**T527**) |
| [`WalletButton.tsx`](../frontend-dapp/src/components/wallet/WalletButton.tsx) | Shell header Connect Wallet — keep |
| [`index.css`](../frontend-dapp/src/index.css) | `.trade-ticket-heading`, `.side-control*` |
| [`AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md`](./AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md) | Radiogroup / escrow / a11y (#153, #155) |
| [`AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md`](./AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md) | Footer CTA stays (#527) |
| [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) | Heading tracks **displayed** base (#524) |
| [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md) | `:focus-visible` on `.side-control` |
| [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md) | Token rules; side fills are the documented exception |

## Invariants (T563-1–T563-8)

1. **T563-1 Full heading** — at default desktop ticket width, `trade-ticket-heading` shows full **Buy {base}** / **Sell {base}** for hub tickers (cLUNC, cUSTC, UST1, USTR). No CSS `truncate` / ellipsis (`Buy c…`). Long symbols wrap (`overflow-wrap: anywhere`); they must not clip.
2. **T563-2 No header wallet chip** — delete the compact ticket-header Connect Wallet / truncated-address pill (disconnected **and** connected). Do not put bech32 in the heading.
3. **T563-3 Connect paths stay** — disconnected users connect via shell header **and** ticket footer **Connect Wallet** (`TRADE_MONEY_CTA_CLASS`). Do not add a fourth connect path. Removing the chip must not remove the only connect path when the header is scrolled off (**A3**).
4. **T563-4 Invert stays** — `trade-ticket-pair-invert` remains `shrink-0` and tappable. Heading + side labels use the **displayed** base after invert (#524). Colors still follow **bid=Buy / ask=Sell**, not “green means token0”.
5. **T563-5 Semantic side colors** — Buy reads green (`side-buy-idle` / `side-buy-selected`); Sell reads red (`side-sell-*`). Selected vs idle is distinguishable without color (`aria-checked` + stronger fill / font-weight). Idle Sell must not look like `alert-error`. Selected Buy must not look like a success toast. Contrast on **dark and light**. `:focus-visible` rings stay on `.side-control`.
6. **T563-6 Shared component** — `/trade` (`idPrefix="trade-ticket"`) and `/limits` (`idPrefix="limit-orders"`, compact) share `LimitOrderBidAskSideSelector`. Limit/Market tabs stay `tab-glass*`. Place/Market CTAs stay `btn-primary` (blue). Order-book Bid/Ask column colors unchanged.
7. **T563-7 Behavior unchanged** — radiogroup + Arrow/Home/End (#153). `side === 'bid'` still escrows token1; `ask` escrows token0. Parents still clear/re-apply amounts on side change (#155). Click Buy still `onSideChange('bid')` (**A1** — a green/red swap is a fund-direction bug).
8. **T563-8 Docs** — this playbook + `docs/frontend.md` + `docs/design-system.md`. Verify: `make verify-issue-563`.

## Rules of thumb

1. **Heading is text nodes** — `{verb} {displayBase}` (no `innerHTML` / `dangerouslySetInnerHTML`). Oversized / RTL / ZWJ symbols wrap; they must not paint over Sell or the footer CTA (**A2**).
2. **Connected identity lives in the header wallet control** — do not replace the chip with a static address the dApp does not own (**A4**).
3. **Theme tokens** — `--side-buy-*` / `--side-sell-*` in `theme-dark.css` / `theme-light.css` (hue from `--color-positive` / `--color-negative`). A `data-theme` flip must not leave white-on-white side buttons (**A7**).
4. **Do not paint primary money CTAs green/red.** Side controls only.
5. **Do not restyle Limit vs Market tabs** to match Buy/Sell.

## Verify

```bash
make verify-issue-563
make lint-frontend
python3 scripts/check_design_tokens.py
```

Issue: [GitLab **#563**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/563) (AC **T1–T8**, attack **A1–A8**).

## Related

- Side selector a11y / escrow: [`AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md`](./AGENTS_FRONTEND_LIMIT_ORDER_SIDE_SELECTOR.md) (#153, #155)
- Footer Connect Wallet: [`AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md`](./AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md) (#527)
- Pair invert heading: [`AGENTS_FRONTEND_TRADE_PAIR_INVERT.md`](./AGENTS_FRONTEND_TRADE_PAIR_INVERT.md) (#524)
- Copy: [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) (#489) — labels stay **Buy {base}** / **Sell {base}**
- Design tokens: [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md)
- Focus rings: [`AGENTS_FRONTEND_A11Y_FOCUS.md`](./AGENTS_FRONTEND_A11Y_FOCUS.md)
